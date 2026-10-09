# Salvo Language Spec — Labeled Rules

A companion to [docs/language/](docs/language/) (the narrative spec). This file
states every language feature as a short, labeled rule, with the compiler's
implementation decisions as sub-bullets. It assumes familiarity with the
language; its purpose is precision and greppability.

Conventions:

* Rule labels are stable identifiers: `[area-topic]`. Compiler code and
  tests reference them in comments (`grep -rn '\[qual-erasure\]'` finds the
  rule, its implementation, and its tests).
* Top-level bullets are *language rules* (what docs/language/ means).
  Sub-bullets are *compiler decisions* (how the implementation realizes the
  rule, including deliberate cuts). When behavior is ambiguous, docs/language/
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
* [bytes-type] (Since 2026-10-04 a value platform type
  [platform-value-type]: `std/platform/core/bytes.{rs,kt}` implements it,
  Rust as `Vec<u8>`, Kotlin as the shipped `salvo.SalvoBytes`; `bytes_of` and
  `mut_bytes` are Salvo over `empty_bytes`, and a `for` over a buffer goes
  through its `BytesYield` pass — the native loop each backend had is gone.)
  **`Bytes` is std's byte buffer** (`core.bytes`, user decision
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
    [iter-for-native] [platform-iterable]; `BytesYield` is the iterator the combinators drive
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
* [interp-nested-str] A **string literal may appear inside `${...}`**, and it
  may interpolate in turn: the fragment scanner skips a nested literal whole,
  honouring escapes, so the literal's own quotes and braces belong to it rather
  than ending the enclosing string. `"${name ?: \"unknown\"}"` is the shape
  that forced it — a text fallback is the most ordinary thing to write in an
  interpolation, and until this the file lexed as an unterminated string.
  * Only the nesting is new: the fragment is still captured as raw source and
    re-lexed by the parser, which is what makes the recursion free.
  * An unterminated nested literal is reported as the *interpolation's* error,
    since that is the construct the scanner was in.
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
* [op-compound] `x += e`, `-=`, `*=`, `/=`: **pure desugar** to
  `x = x op e`, in the parser (user decision 2026-09-22, the heap plan's item 6).
  Four of them, one per two-operand arithmetic operator; `%=` is deliberately
  absent, since a remainder-in-place has no reading a reader would guess.
  * Nothing downstream knows the spelling exists: the operand rules come from
    [op-arith] (so `+=` on a `Str` is refused naming interpolation, and an
    int-float mix is the same explicit-conversion error), and the place rules,
    the narrowing reset [narrow-assign-reset] and the fate analysis all come
    from the assignment it becomes.
  * **The target is duplicated** in the AST — once as the target, once as the
    left operand — which is sound because a Salvo place is an identifier, a
    field path or a subscript: there is nothing in one to evaluate twice.
  * A **statement**, not an expression: no chaining (`a += b += c`), and the
    operator does not cross a line break (`same_line`, as `++` requires).
  * One token each in the lexer, after `++`/`--` so the step operators keep
    their spelling. `/=` cannot collide with a comment: `//` and `/*` are
    consumed earlier.
  * `++` is **not** re-expressed as `+= 1`: it is also an *expression*, where
    the fixity decides the value [inc-dec], so it keeps its own node.
* [op-assign] An assignment's target must be a **place**: a variable, a field
  path (`p.name`), or a subscript (`xs[i]`, whose *index* is an ordinary
  expression). Anything else — a call above all — is an error naming the three
  forms.
  * Before 2026-09-22 a non-place target was accepted and reached the backend,
    where `f(x) = 1` became rustc's E0070: a [backend-never-wrong] violation,
    found while adding [op-compound], which inherits every rule of the
    assignment it desugars to.
  * A **tuple element** is refused by its own rule instead
    ([expr-tuple-index]: `Mut` cannot apply to a tuple, so its elements are
    read-only), so one mistake stays one diagnostic.
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
    `Char`, `Byte`) and `Str`. `Mut Str` never reaches the question: a
    builder is converted first [str-drop-mut].
  * [interp-union] **A union is the text of the arm it holds** (user
    decision 2026-10-06). A `to_str` that takes the union itself is used
    when there is one; otherwise every arm needs a text form of its own — a
    native one, or a `to_str` in scope (with its implicits filled, as for
    any interpolation) — and a missing one is an error naming the arm. The
    IR carries it as one `UnionToStr { value, arms }` node — the value and
    the function that renders each arm — and each backend's `UnionN` has a
    `to_str` (Kotlin `toStr`) taking those functions and calling the one for
    the arm held; `Checked::interp_union` carries the per-arm forms.
* [interp-float] **A float's text is Salvo's rule, which is Kotlin's** (user
  decision 2026-09-25): the shortest digits that round-trip, **always a
  decimal point**, and computerized scientific notation outside
  `[10^-3, 10^7)` — `2.0`, `8.25`, `-2.0`, `0.001`, `1.0E-4`, `1234567.0`,
  `1.2345678E7`, `1.0E20`; `NaN`, `Infinity`, `-Infinity` for the specials.
  `Float` follows the same rule at its own precision.
  * The alternative considered and rejected the same day: always writing the
    number out in full, which reads better at `1.0E20` and turns `1.0E-300`
    into three hundred zeros. Kotlin's threshold is the compromise, and
    adopting it outright means one backend prints it natively.
  * Kotlin needs nothing. **Rust** does: its `Display` writes the full number
    and drops the `.0`, so the same value printed differently on the two
    backends — a [backend-parity] break in program output. The Rust backend
    routes every float that becomes text through a runtime helper
    [rs-float-text], including inside a container (a list of floats is
    rendered element-wise, since the runtime's `Display` can only call
    `Display` on its elements) and a struct field rendered field-wise
    [interp-struct].
  * Floats are not `Set` elements or `Map` keys [col-key-eligible], so only
    the *value* half of a map needs the rule.
  * **Otherwise a `to_str`**, resolved *at the interpolation site* like an
    implicit parameter (user decision 2026-09-11): a `to_str` in scope
    whose parameter accepts the type and which returns `Str`. The winner is
    recorded in `Checked::interp_to_str` and the emitters call it.
  * **std provides** `fn to_str<T>(list: List<T>, ?to_str: (x: T) -> Str)
    -> Str`, rendering `[1, 2, 3]`, in Salvo since 2026-10-04
    [platform-value-type]; and `to_str` for every scalar, `Double` and `Float`
    included (by [interp-float]), so a list of any scalar prints. A `to_str`
    that takes implicits is called from the interpolation with them filled
    at the zero-width span after the value, which no other call occupies.
    The format is the *language's*, because the targets' own collection
    formatting disagrees (Rust `Debug` quotes strings, Kotlin's
    `joinToString` does not).
  * **`params ToStr<T>`** exists as a *convenience* only (user decision
    2026-09-11): declaring `: ToStr<self>` does not enable interpolation —
    a `to_str` in scope does that — it validates at the declaration that
    one exists, which is where the mistake is easier to see.
  * A nested list (`List<List<Int>>`) and a tuple interpolate: the element's
    `to_str` itself takes an implicit, filled in turn [implicit-recursive].
* [interp-struct] **A struct interpolates by opting in** (user decision
  2026-09-28, comptime round 1, replacing the 2026-09-11 default-on
  derivation): `struct Person : ToStr<self> by auto { … }` stamps the
  field-wise `to_str` from `core.auto` — `Person { name: ann, age: 3 }`,
  Salvo's struct-literal shape, identical on both backends and deliberately not
  Rust's `Debug` or a data class's `toString`. With no `to_str` at all, `${p}`
  is an error naming the clause as the remedy. The stamped body interpolates
  each field, so a field of any type that itself renders — a scalar, a float
  by [interp-float], a nested struct with its own opt-in — renders inside it,
  where the old derivation stopped at natively renderable fields.
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
* [union-arm-identity] Union arm identity is *positional over the runtime
  union's non-`None` arms, in order of first appearance* (qualifiers
  erased). For a union without literals the runtime union is the declared
  one, so this is the declared arms in declaration order.
  * **Literals collapse** [type-literal] (user decision 2026-09-30): a
    base type's literals share one runtime arm of that base, at the
    position of the first of them; a non-literal arm whose base is the
    literals' base (`Other Str`) joins them when it is the only such arm —
    two of them (`Ok Str | Err Str | "a"`) keep their wrapper tags, and the
    literals take an arm of their own. A union left with one value arm is
    that arm (`"A" | "B"` is a `Str`; with `None`, a `Str?`).
    `salvo_core::literal` is the one definition (`collapse_ty`,
    `collapse_ast`, `runtime_arm`).
  * The checker reasons over the *declared* union (narrowing and
    exhaustiveness are about the literals); the side tables a backend reads
    (`coerce`, `is_tests`, `repr_ty`) are in runtime arms —
    `literal::lower_tables` rewrites the coercions after the last round,
    and an `is` test is recorded in runtime arms with **value conditions**
    (`UnionTest.values`: the literals an arm must, or must not, equal).
    Every backend's runtime encoding must preserve these indices. Never
    reorder or dedupe in ways that change arm indices, and keep the checker
    and emitters agreeing on the ident-unwrap predicate.
* [type-literal] **A literal is a type** (user decisions 2026-09-30): the
  type whose one value is that literal — `"STANDARD"`, `3`, `-1`, `3L`,
  `true`; a float is a parse error (equality on floats does not name a
  value), and a `Byte` literal has no syntax yet.
  * **Contextual only**: a plain literal expression (no interpolation)
    takes a literal type where the expected type is it or lists it —
    annotations, parameters (overload selection sees the literal),
    assignment to a declared variable including a `T?` one — and is its
    base anywhere else (`let x = "a"` is a `Str`; a literal binding a type
    variable binds the base).
  * **What fits**: a literal the union lists, or a value already of a
    listed literal type. A literal it does not list is an error naming the
    listed values (with `other(…)` offered when the union is open); a plain
    base value must be narrowed first. A value reaches an `Other` arm only
    through `core.other`'s `other(…)` — `Other` is an ordinary
    `provenance qualifier Other<T> of T` with no rules of its own.
  * **Narrowing**: `is "A"` (the check is the literal alone), `is Kms` for
    a named literal sub-union, `is Other` for the rest; `when` over a
    subject is exhaustive over the literals as over any arms.
  * **Widening**: a literal is a subtype of its base, and a union with
    literals is a subtype of whatever its widened runtime shape is
    (`literal::widen`: `"A" | "B" | Other Str` → `Str`) — so it passes as
    its base, unifies as its base, and compares with `==` as its base.
  * **Refused**: overloads apart only by literal types of one base
    ([fn-overload-duplicate]'s erased form: both would be one host
    signature); `"a" | Str` is a warning (subsumed; the open arm is `Other
    Str`).
  * Backends: a literal renders as its base; a test with value conditions
    is a comparison — Kotlin a subject-less `when`/`==`, Rust a guarded
    pattern (`ref __v if *__v == "A"`, the last `when` branch the `_` rustc
    needs).
* [type-nullable] There is no null value: `T?` is shorthand for
  `T | None`. `x!` asserts non-`None` (panics otherwise).
* [assert-op] `expr!` asserts that a value is **present** and answers it
  without its `None` arms. Its operand's type must *have* a `None` arm: on
  anything else the `!` states something false, and it is an error naming the
  alternatives (`?:` for a fallback, `is None` for a test). Before the rule
  (user decision 2026-09-23, ASSERTIONS.md A-1) the two backends disagreed about
  the leftover — rustc refused `.unwrap()` on an `i32` while kotlinc accepted
  `!!` with a warning and ran, which is [backend-never-wrong] broken at the
  checker's expense. `Ty::Unknown` iterators through [type-unknown-lenient].
  * A union with no `None` arm is the same mistake, since `!` removes `None`
    arms and there are none.
  * The rule makes a `!` that *becomes* provable a build error rather than dead
    code — which happens whenever the checker learns to prove more
    ([col-of-nonempty] and [deduce-gained] each did it in one day). That cost is
    accepted deliberately: a `!` whose operand cannot be absent is a statement
    that is false, not merely unused.
* [assert-fn] Two compiler-owned forms, spelled with a `!` because a bang in
  Salvo marks a place that can fail (user decision 2026-09-23, A-3):
  `assert!(cond)` / `assert!(cond, "why")`, and `unreachable!()` /
  `unreachable!("why")`.
  * `assert!` has type `None` and continues when the condition holds;
    `unreachable!` has type **`Never`**, so it stands where any value is
    expected and ends the path exactly as `throw` and `return` do
    [expr-escape] [fn-must-return].
  * The condition is a `Bool` [cond-bool] and the message a `Str`.
  * **Contextual**: `assert` and `unreachable` stay ordinary identifiers, and
    only `assert!(` / `unreachable!(` are the forms.
  * **Why not library functions**, which is what a reader expects from the call
    syntax: a function could not do any of the three things these do — the
    message is composed *only on failure* (an argument would be built on every
    success, interpolation and all), the condition's narrowing reaches the
    enclosing scope ([assert-narrow], which needs the *syntactic* test, not a
    `Bool` value), and neither name can be shadowed or renamed. The call shape is
    kept because it reads like one; the semantics are the compiler's.
* [assert-narrow] When an `assert!`'s condition is a narrowing test — `is` on a
  type, a qualifier or a union arm, or their `&&` chains — the **then-narrows are
  installed permanently** for the rest of the scope, exactly as the guard idiom
  installs its else-narrows [is-narrow-guard]. So `assert!(v is Int)` makes `v`
  an `Int` below, and `assert!(xs is NonEmpty)` makes `first(xs)` answer an
  element [col-nonempty]. This is what the form is *for*: an assertion that
  informs the type rather than merely checking.
* [assert-trap] A failed assertion is **Salvo's** failure, not the host's: the
  text is `salvo: <what> at <module>:<line>:<col>`, identical on both backends
  (user decision 2026-09-23, A-2), where `<what>` is the written message when
  there is one and `value is absent` / `assertion failed` / `unreachable`
  otherwise.
  * The **location is the module path**, not the file's display name: a name
    depends on how the file was loaded (`std/core/list.sv` from the embedded
    library, `core/list.sv` from a directory walk), and emitted output must not
    depend on the loader.
  * The **mechanism stays each host's own trap** — a panic on Rust
    [rs-assert-trap], an `AssertionError` on Kotlin [kt-assert-trap] — since
    neither program is meant to continue. Only the text is the language's.
  * **Always on, in every build** (user decision 2026-09-23, A-4). Salvo has no
    build modes, and every language surveyed that made assertions optional
    (Java's `-ea`, Kotlin's inherited `assert`) ended up with a vocabulary nobody
    trusts.
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
  * **One pick per `?:`.** A *chain* of picks (`r ^Ok?: Err?: err(_)`), where
    only the last one carries a right-hand side and each earlier arm iterators
    through as a value, is **not built** — and is recorded as a question rather
    than a plan (ROADMAP.md, "Pick chains"): the same program is expressible
    with one pick and a `when`, so the case for the sugar wants a real site that
    reads worse without it. The multi-arm work it would have needed is done
    [rewrap].
  * **The subject is evaluated once**, into a temporary registered exactly as a
    hoisted `is` subject is [is-bind-once] — so, unlike `?.`, the subject need
    not be a place.
  * **A pick that matches nothing** can never run; one that matches
    *everything* leaves the right side dead. Both are errors, the same "dead
    scaffolding" refusal a throw-free `try` gets [try]. A pick naming a *type*
    rather than a qualifier is an error too.
  * **Either side may span several arms** (2026-09-21): a multi-arm side is a
    *sub-union* of the storage, produced by the arm mapping [rewrap]. So
    `attempt(t) ^Ok?: return _` works over `Ok Int | Err Str | Thrown Str`,
    where `_` is the two-arm `Err Str | Thrown Str`.
* [pick-qualifies] A predicate qualifier on a **non-union** subject is the
  pick's *runtime* form (user decision 2026-09-24) — the same duality `is`
  has between arm identity [is-narrowing] and a `qualifies` call
  [is-qualifies], reached from `?:` instead of a condition:

  ```
  let i_child = i * 2 + 1 Idx(heap)?: break
  // i_child: Idx(heap) Int
  ```

  The subject is evaluated once into the pick's temporary, `qualifies` is
  called on it (dependent slots filled exactly as an `is` fills them
  [qual-depend], constants included [qual-const]), and when the claim holds
  the temporary *is* the value — claimed. When it does not, the right side
  runs; `_` there is the plain subject.
  * **`^` is refused**: a predicate pick establishes; the subject carries
    no tag to lift off. A constructive qualifier is refused too — no
    runtime test exists, so nothing could decide the pick
    [qual-constructive].
  * **A value-yielding right side drops the claim** unless it carries it:
    a claim and its absence are one representation, never two union arms —
    `5 Idx(xs)?: 0` is an `Int`. A diverging right side leaves the claimed
    type, and narrows a place subject below [elvis-guard].
  * `qualifies` effects must be available at the pick, as at any test site
    [is-qualifies-effects].
  * Lowered as the elvis conditional with the predicate call as its
    condition, on both backends; the claim itself erases [qual-erasure].
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
* [elvis-guard] **A leaving right side takes its flow effects with it**: a
  `return elem` there moves `elem` on a path the fall-through never shares,
  so its consumptions are restored at the join (found on `std.heap`'s
  sift, 2026-09-24 — the pick's `?: return elem` poisoned the fn's own
  final `return elem`). A value-yielding right side merges its effects as
  ever, since both paths continue.
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
* [rewrap] A value spanning **fewer arms than its storage** is produced by
  mapping arm to arm: each source arm to the target arm of the same type, and a
  source arm the target does not have to an unreachable. This is what
  [let-infer]'s `Rewrap` coercion has always emitted for an annotated slot
  (`let b: Str | Int = a` out of a `Str | Int | Bool`); since 2026-09-21 the
  sites that produce such a value **with no slot** reach the same mapping:
  * a **multi-arm lift binding** [qual-lift], whose source arms are recorded
    *lifted* — the mapping pairs arms by type equality, and a lifted arm's type
    is the arm without its qualifier, while positions stay the storage's since
    qualifiers erase;
  * a **pick's picked side**, same treatment;
  * a **pick's unpicked side** (`_`), whose source arms keep their tags, because
    `_` never lifts [pick].
  * The mapping **consumes** its input, so it takes an owned copy: the subject is
    still readable after the form ([elvis-guard], and the arms the right side
    reads). Kotlin needs no copy, its values being references.
* [placeholder] `_` reads as **the value the enclosing construct left
  unnamed**, and **no construct binds one yet** (user decision 2026-09-21): a
  plain `?:` leaves `None`, which the program can already write, so a
  placeholder there would name a value that has a name. It earns its keep where
  a *qualifier* is picked and the unpicked side carries a tag (step 5), which is
  where `x Ok?: err(_)` needs it.
  * It is not a name: it cannot be declared, shadowed or captured. The word is
    reserved, and appeared in no `.sv` source when it was.
  * **A `for` binder may be `_`**: `for _ in range(0, 3) { … }` drives the iterator
    and binds nothing, which is how a loop that repeats *n* times is written.
    The element is still produced — the `Finished` arm is what ends the loop —
    so only the binding goes. Each backend renders that in its own terms: Rust
    binds the wildcard pattern (`mut _` is not a binding it accepts), and Kotlin
    drops the `val` line for an iterator loop and names an unreachable local for a
    native `for`, which has no wildcard form. Two such loops in one function are
    independent, since neither introduces a name to collide.
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
* [checked-type] **`Checked<T>` is std's "you must look at this"**: a
  `linear struct` in `core.checked` wrapping a value whose obligation is
  discharged by `detach` (read it) or `ignore` (say you meant not to). Dropping
  one is the ordinary linear error [linear-group], which is the whole mechanism —
  no new rule, just the obligation applied to an *answer*.
  * In `core` because std's own returns carry it ([col-bounds]'s `swap`, the
    whole filesystem's `Err Checked<FsError>` [fs-surface]), and a type in a
    returned position must be visible wherever the function is.
  * **Looking without discharging is a field read**, `e.value` — the obligation
    stays owed, which is what the spelling says. There is no
    `to_str(Checked<T>)`: it would need the element's own `to_str` as an
    implicit, which waits on recursive implicit resolution (user, 2026-09-26).
  * `ignore` refuses a **linear** `T`: discarding the wrapper would discharge the
    inner obligation too, which nothing has looked at. `detach` takes
    `<T canbe linear>` and hands the obligation on.
* [col-bounds] **An index-taking operation answers; it does not fail.** An
  out-of-range index is an ordinary outcome in Salvo, reported the way the
  operation's own shape allows, and identically on both backends — never a panic,
  an exception, or a wrapped-around read. (The label was referenced by
  `core.list` before it was written down; defined 2026-09-22 with `swap`.)
  * A **read** answers an optional: `get` on a list, an array or a `Bytes`,
    `char_at` on a `Str`, `slice`/`substring` for a range. Both ends count, and a
    negative index is out of range rather than counted from the back.
  * A **write answers a `Bool`** — `false` when the index is out of range, and
    then nothing was written (user decisions 2026-09-22): `swap(list, i, j)`,
    `set` on a `Mut Str`, `set` on a `Mut Bytes`. Chosen over a silent no-op
    because a write that quietly did nothing has no symptom at the call site, and
    over the hosts' behaviour because they disagree — `Vec::swap` panics where a
    JVM list throws, and `setCharAt` throws where a slice write would panic — so
    the same program would fail differently per backend [backend-parity]. The
    answer is ignorable, and at a known-good index that is what a caller does.
  * **A list write's answer is a `Checked<Bool>`** (user decision 2026-09-26):
    `swap` hands back an obligation, so the answer cannot be dropped by accident
    — `detach` it to read, `ignore` it to say the miss was expected
    [linear-group]. The failure this reports is precisely the one with no symptom
    at the call site, which is what the wrapper exists for. The **total** `swap`
    overload — both indices carrying `Idx` claims — answers plain `None`: there
    is no failure to check, and it discharges the inner call's obligation itself.
    `set` on a `Mut Str`/`Mut Bytes` keeps the bare `Bool` for now; the list is
    where the surface was reshaped.
  * An out-of-range write **never grows the value**: that would make a `set` an
    `append`, and the buffer is the caller's.
  * There is **no positional write for a list** at all [linear-container]: the
    displaced value would have nowhere to go when the index misses, and every
    available answer either drops it or confuses "displaced" with "bounced".
    `swap` is the exchange that escapes the question by moving nothing in or out.
  * **A literal index is not a special case** — which it was until 2026-09-22 on
    the Rust backend: the lowerings cast straight to `usize`, a literal takes its
    type from the cast target, and `(-1) as usize` is rustc's E0600. So
    `get(xs, -1)` type-checked and then failed to *build*, with no Salvo
    diagnostic ([backend-never-wrong]); a variable holding `-1` was fine. The
    cast now goes through `i64`, and one e2e case per backend from verbatim one
    source pins every index in this rule.
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
* [col-of-nonempty] A collection constructor comes in **two shapes** (user
  decision 2026-09-23): an empty one, and one whose first element is a
  *required* parameter and whose result therefore claims `NonEmpty` by
  construction.

  ```
  export intrinsic fn list_of<T canbe linear>() [] -> List<T>
  export intrinsic fn list_of<T canbe linear>(first: T, ...rest: T[]) [] -> +NonEmpty List<T>
  ```

  So `list_of()` is empty, `list_of(1, 2, 3)` and `list_of(a, ...rest)` are
  `NonEmpty`, and nothing is checked at run time [qual-ctor-predicate]. Built
  for `list_of`/`mut_list_of`, whose `NonEmpty` is declared in the same file
  [qual-ctor-same-file]. `set_of`/`map_of` and the sorted pair keep the single
  variadic shape: their `NonEmpty` now lives beside them (2026-09-26), so the
  two-shape convention is *available* to them and adding it is an std API
  question rather than a blocked one (ROADMAP.md).
  * A **lone `...spread` reaches neither shape**: a spread may not supply a
    required parameter [fn-variadic], and the empty shape takes no arguments.
    An array becomes a list through `map_to` or a loop with `add`.
  * Consequences worth knowing, all from the claim being *in the type*: a
    variable inferred from a constructor call is `NonEmpty`, so assigning a
    plain list to it later is refused (annotate to widen); a constructor call
    does **not** fit a plain type-*argument* position
    (`List<NonEmpty List<Int>>` is not a `List<List<Int>>`, since type
    arguments are invariant), where a **literal** does — `[[1, 2], [3]]` claims
    nothing; and `first` on the result answers an element, so a `!` after it
    is redundant.
  * A **literal** (`[1, 2, 3]`) claims nothing, deliberately unexamined so far:
    the convention is about the constructors (ROADMAP.md records the question).
* [col-nonempty] std declares `qualifier NonEmpty<T> of List<T>` in
  `core.list` (2026-09-13), with a `qualifies` of `size(list) > 0`, the
  by-construction constructor of [col-of-nonempty], and a
  `first(list: NonEmpty List<T>) -> proj(list) T` overload that drops
  the optional — ranked above the plain `first` by [fn-overload-rank].
  * A **refinement** `refn add(list: Mut List<T>, elem: T) => list: +NonEmpty`
    establishes the claim, because `add` itself may not [qual-refn]. So does
    `refn swap(list: Mut List<T>, i: Int, j: Int) => list: +NonEmpty`
    (2026-09-23): exchanging two elements cannot change how many there are, and
    `swap`'s own exhaustive clause has to strip the claim like every mutator's.
    That refinement is what lets `std.heap`'s sift-down keep a `NonEmpty` it was
    handed — the mechanism for "keep a claim across a call that has never heard
    of it" is a refinement written by the claim's **owner**, never a wrapper
    overload at the call site.
    Consequence for user code: another qualifier refining `add` over a `List`
    *disagrees* with std's, so the call is refused until it names the place it
    means [qual-refn-ambiguous]. Two remedies, both named by the diagnostic:
    `add@place(...)` per call, or one word — `with NonEmpty` on the user's
    qualifier — which makes the two claims co-applicable and removes the
    disagreement altogether.
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
* [col-sorted-list] std declares
  `qualifier Sorted<T>(?cmp: (T, T) -> Int) of List<T>` in
  `core.list` — a **state claim** over a list, and a different mechanic from
  the `SortedSet`/`SortedMap` types [col-sorted], which are a representation.
  A `Sorted List<T>` still reaches the whole list surface.
  * **The claim names the ordering it was sorted by** (user decision
    2026-09-22, the ordering round's decision 22): orderings are plural [cmp-groups], so
    "sorted" alone does not say enough — an `add_sorted` under a different `cmp`
    than the sort used inserts at a position that is a lower bound for one
    ordering and nonsense for the other. The slot is the same mechanism a
    structure that *holds* an ordering uses [cmp-carry], and the identity is the
    only thing in the claim: a qualifier erases, so nothing holds the comparator
    at run time and it arrives at each operation as the implicit parameter the
    binder lowers to [cmp-binder].
  * **No `qualifies`**, so no `is Sorted`: deciding whether a list happens to
    be sorted compares its elements, which nothing can do over an
    unconstrained `T`. It is minted by `sort` / `mut_sort` and nowhere else.
  * `sort(list: List<T>, ?Ordered<T>) -> +Sorted<T>(?cmp) List<T>` (and
    `mut_sort`) **publishes** what resolution found; `add_sorted` and
    `binary_search` **capture** it from the argument's type, so each is computed
    with the ordering the list is actually in. Before this, both backends sorted
    and searched by the *host's* ordering — Rust's derived `Ord`, Kotlin's
    `__salvoCompare` — so a hand-written `cmp@Person` was ignored by all three.
  * `add_sorted(list: Mut Sorted<T>(?cmp) List<T>, elem: T) => list: Mut Sorted, !elem`
    inserts at the position that keeps the order, and names `Sorted` in its
    **own** exhaustive deduction list. Its parameter *demands* the claim, since
    inserting in order into an unordered list would not make it ordered.
  * `binary_search(list: Sorted<T>(?cmp) List<T>, elem: T) -> Int?` is honest
    only because of the parameter's claim. With equal elements both backends
    answer the **lowest** matching index, and "matching" is a **tie in that
    ordering** (`cmp(a, b) == 0`), never the host's equality [col-membership]:
    each lowers to an explicit lower bound (Rust `partition_point`, not
    `Vec::binary_search`, which may answer any index in an equal run; Kotlin an
    `indexOfFirst`).
  * **The four are ordinary Salvo fns over three private primitives**
    (`sort_by`, `insert_sorted_by`, `search_sorted_by`), because no
    `intrinsic fn` takes an implicit parameter: a lowering is a template over
    rendered arguments, so the binding happens in Salvo and the primitive is
    handed the comparator as an ordinary fn-typed argument. The alternative —
    teaching the intrinsic path to pass implicit arguments — was rejected as
    emitter surgery for a three-line body. `insert_sorted_by` carries a
    module-scoped `refn` re-establishing `Sorted` [qual-refn], which is what
    lets `add_sorted` promise the claim across the mutation; the qualifier has
    no `qualifies` to host it, so it is written top-level [qual-refn-scope].
  * The elements must be **orderable**, on the same terms a `SortedSet` key is
    [col-key-eligible] — checked where the claim is written, and where a call
    infers it (a constructor's `+Q` lives beside the return type rather than
    in it, so the inferred path re-applies it before checking).
* [col-distinct] std declares `qualifier Distinct<T> of List<T>` in
  **`core.set`**, not `core.list`: a constructor must sit beside its qualifier
  [qual-ctor-same-file], and a *set* is what can honestly promise the claim —
  `to_list(set)` answers `-> +Distinct List<T>`. Mint-only, like `Sorted`.
  `to_list` over a `SortedSet` lives in `core.sorted` and so cannot mint it.
* [col-reversed] `reversed(list)` walks a list back to front — an **iterator**
  [iter-protocol], not a copy: `: Yield<self, proj T>`, so `for x in
  reversed(xs)` borrows each element [yield-proj] and `xs` stays usable.
  Kotlin's own `reversed()` answers a fresh list; a Salvo caller wanting the
  reversed *list* writes `to_list(reversed(xs))`. Step 0 of the
  refinement-types sequence (user decisions 2026-09-23; ROADMAP.md).
* [col-enumerate] `enumerate(list)` and `enumerate_rev(list)` pair each
  element with its index — ascending from `0`, or descending from
  `size(list) - 1` to `0`, which is the descending index loop with the
  element already in hand. The element is `Enumerated<T>`
  (`{ index: Int, elem: proj T }`), a **view struct** rather than a tuple:
  a qualifier cannot apply to a tuple [qual-union-arm] and a tuple literal
  cannot *store* a projection ([fate-derived-readonly] — probed 2026-09-23:
  the store is a move, and `copy` would charge every step), while a `proj`
  **field** is the declared lend the pair needs [proj-field]. One pass
  struct serves both directions, stepped `+1` or `-1`.
  * The iterator's `next` writes the opaque lend (`-> Emitted
    Enumerated<T> | Finished holds proj(p)`) by hand: the result holds the borrow inside a *generic instantiation*
    (`Emitted Enumerated<T>`), which body inference cannot see through a
    generic constructor — the exact case the written form exists for
    [proj-infer].
* [col-idx] `core.index` declares `qualifier Idx<C canbe linear>(c: C) of
  Int` [qual-depend] — `0 <= index < size(c)`, about one particular
  container — with **total overloads** consuming it: `get(list, index:
  Idx(list) Int) -> proj(list) T` (no `None` arm), `swap(list, i: Idx(list)
  Int, j: Idx(list) Int) -> None` (no `Bool` — the claims did the checking
  [col-bounds]), `replace` [col-replace], and the same `get`/`replace` on a
  `Deque` and the buffers [buffer-type]. Ranked above their plain siblings
  [fn-overload-rank].
  * **Any container with a `size`** (user decision 2026-10-05, ROADMAP §0j
    6g0; built 2026-10-05): `qualifies(index: Int, c: C, ?size: (c: C) ->
    Int)` takes the `size` as an implicit, filled where the claim is tested
    at the type the slot binds (`check_predicate_quals` unifies the slot with
    the place and calls `fill_implicits` at a zero-width span on the first
    value argument, `PredicateCheck::implicits_at`, where both emitters read
    the call's implicit arguments). Nothing prints in the type. A
    `qualifies` may take implicits generally: they trail, and the shape check
    counts the others. Rust reborrows a slot argument that is already a
    reference (`&*heap`), so a generic slot does not bind to the `&mut`.
  * Its own module, with `NotEq` [col-noteq], so the refinements that keep
    it — a list's `add`, `swap` and `replace`, a deque's `add_first`,
    `add_last` and `replace`, a buffer's `replace` and `clear` — live beside
    it [qual-refn-scope]. A refinement's own type parameters take the leading
    positions when it is matched [qual-refn-match] (`refn add<T>(list: Mut
    List<T>, …)` inside `Idx<C>`). The writes it refines are exported
    platform fns or Salvo fns, since the owner has to see them, and
    `+Idx` is trusted only in `core.index`, so `indices`/`rev_indices` live
    there and `core.list`'s searches return `(Idx(list) Int)?` proven by
    `indices` or a test.
  * The **total read is the host's own** (`get_at`), not the optional `get`
    and `!`: a `T?` with `T` optional flattens on the JVM, so `!` read a
    stored `None` of a `List<Str?>` as absence and trapped (the recorded
    defect, fixed 2026-10-05). Same on `Deque`.
  `core.map`'s `KeyOf` has its total `get` too (landed with `preserve`
  [qual-preserve], which is what lets a claim survive the `put`s between
  the test and the read). **The index iterators mint the claim**:
  `indices(list)` and `rev_indices(list)` emit `Idx(list) Int` elements —
  `for i in rev_indices(xs) { get(xs, i) }` is the founding example of the
  refinement-types design, total end to end — and `binary_search`'s found
  arm is `(+Idx(list) Int)?`, so narrowing the optional is the last check
  the result ever needs.
* [buffer-type] **`core.buffer`'s `IntBuffer` and `LongBuffer`** (user
  decisions 2026-10-05, ROADMAP §0j 6g; built 2026-10-05): `iterable platform
  type IntBuffer canbe Mut : Iter<self, Int>`, a fixed-length run (Kotlin
  `IntArray`, Rust `Vec<i32>`; `LongBuffer` over `Long`, `LongArray`,
  `Vec<i64>`). `int_buffer(size, fill)`/`long_buffer`, `size`, `get(buf,
  i) -> Int?` and the total `get(buf, i: Idx(buf) Int) -> Int`, `replace(buf,
  i: Idx(buf) Int, v) -> Int`, `clear(buf, fill)` (both keep `Idx` claims),
  `to_str` (`[1, 2, 3]`) in Salvo, and `iter` for a generic `Iter` fn. No
  add or remove; a plain buffer is immutable and `copy` copies the array
  (Kotlin `copyOf`); `noremote` like every platform type; no `==`. The
  operations are Salvo over private `int_*`/`long_*` platform fns, since two
  platform fns of one module may not overload.
* [col-noteq] `core.index` declares `qualifier NotEq(i: Int) of Int with
  Idx` [qual-depend] [qual-with] — `j != i`, bound to `i`'s identity — the
  proof that two element handles of one list cannot alias [elem-distinct].
  Established by the ordinary filled test (`j is NotEq(i)`), stripped by
  mutation *or reassignment* of either side [qual-depend]. Named for what
  it claims (user decision 2026-09-24: `NotEq` over `Distinct`, which
  `core.set` already uses for a `List` subject — and a `NotEq` over any
  `?eq`-capable subject is the recorded generalization, ROADMAP). Ships
  ahead of the `update2` family (ladder step ③), which consumes it.
* [col-update] `core.list`'s **in-place update family** (step ③ of the
  group-borrowing ladder, user decisions 2026-09-24): `update(list:
  List<Mut T>, index: Idx(list) Int, f: (elem: Mut T) -> None)` hands the
  callback the mutable element handle [proj-mut] — nothing copied, moved
  out, or put back — and `update2(list, i: Idx(list) Int, j: NotEq(i)
  Idx(list) Int, f: (a: Mut T, b: Mut T) -> None)` is the two-element
  transaction, its indices proven apart by the declared claim
  [col-noteq] [elem-distinct]. Both promise `preserve Idx`
  [qual-preserve]: an in-place write moves no boundary, so sequential
  updates stay total. **Ordinary Salvo, not intrinsics** — the bodies are
  exactly the mints the proofs legalize, riding [rs-loc] on the Rust
  backend. Note the parameters carry no container `Mut`: element
  mutability is the element type's [proj-mut].
* [col-locate] `core.list` declares `params Locate<C, L, T> { fn at(c: C,
  l: L) -> proj(c) Mut T? }` (user decision 2026-09-24, ④a slice 3′): what a
  **position-based** algorithm needs, as a params group [implicit-group] —
  one function turning a container and a position into the element's
  mutable handle, so the algorithm stays generic over *what a position is*
  (an index for a list, a key for a map, a cursor of your own) while the
  caller, which knows the shape, fills it. The `Yield` pattern for places
  rather than elements, and the idiom that pierces generic opacity for
  mutable lends. std ships the canonical `at` for a list (`get` under the
  group's name). Rust renders such a position as a **locator** [rs-loc].
* [col-salvo] **Most of the list surface is Salvo** (2026-10-03, user
  request): over the intrinsics `get`, `size`, `add`, `swap`, and two new
  ones, `core.list` writes `last`, `is_empty`, `remove_front(list, n)` /
  `remove_back(list, n)` and `remove_front_while(list, keep)` /
  `remove_back_while(list, keep)` (each answering the removed elements as a
  `Mut List<T>`, in list order, and `canbe linear`), `sub_list(list, from, to)`
  (a clamped copy, by `?copy`: a list of borrows cannot be answered from a
  generic fn on Rust yet, and there is no slice type), `find_first` /
  `find_last` (an index carrying `Idx(list)`), `index_of` / `last_index_of` /
  `contains` (by `?Eq<T>`), `any`, `all`, `count(list, pick)`, `partition`
  (two copied lists, since a tuple cannot hold borrows) and `reverse` (in
  place). A new backend implements none of them.
  * [col-insert] `insert_at(list, index, elem) -> T?`: `index` may be the
    size (append); out of range, nothing moves and `elem` comes back, so a
    list of obligations cannot lose one.
  * [col-remove-range] `remove_range(list, from, to) -> Mut List<T>`: the
    elements in `[from, to)`, clamped to the list, moved out in order.
  * [col-replace] `replace(list, index: Idx(list) Int, value) -> T`: the
    **total positional write** (user decision 2026-10-05, ROADMAP §0j). The
    proven index leaves no out-of-range case, so the displaced element is
    always handed back, and `Idx` claims survive [qual-preserve]. Taking an
    element out of a `List<T?>` is `replace(xs, i, None)`. It takes a linear
    element type too, since a **qualifier's type parameter may `canbe
    linear`** (user decision 2026-10-05): `qualifier Idx<C canbe linear>(c:
    C) of Int` claims an index of a list of obligations. Since 2026-10-05 a
    platform fn of `core.list` (the host writes; `core.index`'s `Idx` refines
    it to keep the claims) [col-idx].
* [str-mut-results] **A list made from a string is the caller's own**:
  `split` and `lines` answer `Mut List<Str>` (2026-10-03, user request).
* [str-search] `index_of(str, needle, from)` (from an offset, below 0 the
  start), `last_index_of(str, needle)`, `replace(str, from, to)`,
  `trim_start`, `trim_end`: intrinsics, since each is one host call. Indices
  count characters on Rust, as `index_of`'s always did.
* [str-salvo] Written in Salvo over those: `is_empty`, `repeat(str, n)`,
  `lines(str)` (split at `\n`, a `\r` before it dropped, no empty last line
  for a trailing newline), and `split_once(str, sep)` / `split_last(str, sep)`
  (`(before, after)?`, at the first or last `sep`).
* [path-type] **`fs.path.Path` is a path as a value** (user decisions
  2026-10-04, and 2026-10-05 for the module, which moved from `path` to `fs.path`: a struct rather than functions over `Str`, so `Str`'s surface
  and its completion stay free of path operations): `struct Path : Hashed<self>
  by auto { text: Str }`, built with `path(text)`, read with `to_str`. Its
  operations are Salvo: `join(p, child)` (an absolute child replaces, a
  trailing `/` is not doubled), `parent` (`None` above the top; `parent(/a)` is
  `/`), `file_name`, `extension` and `stem` (a lone leading dot is not an
  extension), `with_extension`, `segments` (empty ones dropped) and
  `is_absolute`. Separators are `/` on every backend. Nothing touches a disk.
  **`fs` takes paths only as `Path`s** (user decision 2026-10-05): the `Fs`
  members, the one-shots, `MemFs`, `RestrictedFs(root: Path)`. The host layer
  below (`RawFs`) and the `FsError` structs keep text: the first is the
  platform boundary, the second a report.
* [col-span] `core.string` declares `struct Span { start: Int, end: Int }`
  — a struct, not a tuple, because a qualifier cannot apply to a tuple
  [qual-union-arm] — and `qualifier SpanOf(str: Str) of Span`
  [qual-depend]: `0 <= start <= end <= size(str)`, the parse-don't-validate
  pattern for a multi-part precondition (the pair is claimed *whole*, so
  the cross-field fact rides along). `substr(str, at: SpanOf(str) Span) ->
  Str` is total. The `Bytes` sibling waits on same-name-different-subject
  value slots (a second `SpanOf` over `Span` would be a duplicate
  [qual-overload]; recorded in ROADMAP.md).
* [col-deque] **`Deque<T canbe linear> canbe Mut` is std's double-ended
  queue** (user decision 2026-10-02, runtime D3), `core.deque`: O(1) at
  both ends. Surface: `deque_of`/`mut_deque_of` (both shapes) and
  `deque_by`/`mut_deque_by` [col-by]; `add_first`, `add_last`,
  `remove_first`, `remove_last` and `remove_at` (O(n)) moving elements in and
  out [linear-container]; `get`, `first`, `last` answering borrows;
  `size`, `drain`, `to_str`, `to_list`/`to_deque` [col-convert], `iter` over
  a `DequeYield` pass and `reversed`. Out-of-range is `None` [col-bounds].
  * **Not included, by decision**: a literal (`[…]` stays a `List`),
    `NonEmpty`/`Sorted` claims, `eq`/`hash`/`cmp` (not a key), and — for now
    — a wire form: declared `noremote` [noremote], since neither runtime has
    a codec, so a crossing is refused by the checker rather than by the host
    compiler.
  * Lowering: `VecDeque<T>` ([rs-deque]) and `kotlin.collections.ArrayDeque`
    ([kt-deque]), one host type for `Deque` and `Mut Deque` on both. A
    `copy` on Kotlin is always a new deque, since a plain `Deque` may be an
    object someone else holds as `Mut`.

  order, on every backend** (user decision 2026-09-12) — with
  `LinkedHashMap`'s exact semantics: writing a key that is already present
  keeps its original position, and removing one is O(1) and leaves the order
  of the rest intact. `to_list` on a set, `keys` on a map, a `for` over
  either, and `to_str` all agree on that order.
  * **std's host files implement it, the same way on both backends**
    (2026-10-06, ROADMAP §0j step 7; it used to be `LinkedHashSet`/
    `LinkedHashMap` on Kotlin and a runtime file on Rust): a slot list in
    first-insertion order with tombstones, each entry's digest cached, and a
    bucket index from digest to slots, compacted when the tombstones outgrow
    the live entries [platform-slots].
  * Two alternatives were rejected. **Unspecified order** — what
    `HashMap`/`HashSet` give — would make a program's output depend on its
    backend, which [backend-parity] forbids. **Always sorted** would charge
    every collection an ordering it may not need, and would demand orderable
    keys where hashable ones suffice; that is what `SortedSet`/`SortedMap`
    are for [col-sorted].
* [col-membership] **Every keyed container names the basis of its membership,
  and the two bases are different** (user decision 2026-09-22):
  * a `Set<T>(?hash, ?eq)`'s members are **`eq`-distinct** — it buckets by `hash`
    and confirms a bucket hit by `eq`, which is why `Hashed<T>` declares the pair
    [cmp-groups]. A `Map`'s keys likewise.
  * a `SortedSet<T>(?cmp)`'s members are **`cmp`-distinct**: two elements are one
    member when `cmp(a, b) == 0`. A `SortedMap`'s keys likewise. Equality plays no
    part, which is not a shortcut but what both hosts do —
    `BTreeSet`/`BTreeMap` decide duplicates by `Ord` alone, and
    `java.util.TreeSet`/`TreeMap` by the comparator they were handed (the JDK
    documents being "inconsistent with equals" as permitted) — and it is the only
    rule implementable over a BTree.
  * So a `SortedSet<Person>(by_age)` keeps one person per age, by definition. The
    two bases may disagree and that is not a contradiction: `Set<Person>` asks
    "the same person?" and `SortedSet<Person>(by_age)` asks "the same rank?".
    Where a program needs both readings it holds both containers, which is what
    keeping `eq` out of `Ordered<T>` is for.
  * The slot is part of the type, so **membership depends on it**: the same values
    collapse differently in `SortedSet<Person>(cmp@Person)` and
    `SortedSet<Person>(by_age)`, and neither is the other
    [cmp-carry].
* [col-keyed-slots] The four keyed containers declare their slots:
  `SortedSet<T>(?cmp: (T, T) -> Int)`,
  `SortedMap<K, V>(?cmp: (K, K) -> Int)`,
  `Set<T>(?hash: (T) -> Long, ?eq: (T, T) -> Bool)`,
  `Map<K, V>(?hash: (K) -> Long, ?eq: (K, K) -> Bool)`.
  * **The constructors ask for the identity as a capability** (user decision
    2026-09-26): `set_of<T>(...elems: T[], ?Hashed<T>) -> Set<T>(?hash, ?eq)`,
    and the same for the other four families' constructors — so what a container
    is keyed by is decided where it is *built*, and a hand-written `hash`/`eq`
    is honoured instead of ignored. Before this nothing filled the slots, and
    the two backends disagreed about the same program: Kotlin keyed by the JVM's
    structural equality (a declared `eq` silently unused) and rustc refused it.
  * **The identity is passed, never inferred from an annotation**: the program
    writes `mut_set_of(hash = age_hash, eq = same_age)`. An annotation that
    names a pair must *agree* with what the constructor resolved rather than
    choosing it, because [cmp-binder] says what filled the binder decides what a
    structure carries — and one way to choose an identity beats two. The
    expected type does bind the callee's ordinary type *parameters* before its
    implicits resolve (`let s: Mut Set<Str> = mut_set_of()` learns `T = Str`,
    without which every `eq` in scope matches `?eq: (?, ?) -> Bool`), and
    deliberately not its slots.
  * **An unwritten slot is resolved by its name** [cmp-carry] [implicit-resolve],
    so nothing is materialized for it: `Set<Str>` is exactly the type it has
    always been and only a container that says something *different* about its
    keys grows an argument to say it in.
  * **One limitation, accepted deliberately** (user, 2026-09-26), refused with a
    Salvo diagnostic rather than a target-compiler one: a **tuple or a list**
    cannot be a keyed container's subject, because `core.compare` declares
    identities for the intrinsic scalars only and [implicit-resolve] skips a
    candidate that itself needs implicits. This is the same gap that stops
    `(1, 2) == (1, 2)` from resolving, and it did not show before because a
    keyed container over a tuple reached the host's structural comparison
    without asking anyone. ROADMAP's "Recursive implicit resolution" is the
    lift.
  * **The identity is passed per call, never stored** (user decision
    2026-10-06, ROADMAP §0j step 7): every operation receives the `hash`/`eq`
    or `cmp` its call resolved — at a concrete type the canonical or written
    one, inside a generic fn the one it was handed — so a keyed container
    built in a generic body needs nothing special, and one host
    representation serves every identity [platform-slots]. (Until then Rust
    keyed by marker types or by stored `Arc`s and Kotlin by `LinkedHashMap`
    or a runtime slab, chosen per identity.)
* [col-literal-arg] **A bare collection literal as an argument determines the
  callee's type parameter** (fixed 2026-09-25): `to_set([1, 2])` reads `T` off
  the literal's own elements. An expected element type only helps when it is
  *concrete*, and the case arrived in a form the guard did not recognise —
  substituting an **unbound** variable yields `Unknown`, so the pattern
  `List<T>` reached the literal as `List<Unknown>`, looking concrete; the
  literal then adopted `Unknown` as its element type and discarded what its
  elements said. Both the argument's expected type and the element-type read
  now treat `Unknown` as not concrete. An *empty* literal still needs its type
  from the position, which is the rule this left standing [col-literal].
* [struct-literal-arg] **A bare generic struct literal determines its own type
  arguments from its field values** (fixed 2026-09-26): `unwrap(Box { value: 7 })`
  reads `T` off the field, where before it was "no matching overload for
  `unwrap(Box)`" — the literal's type carried no arguments at all, so nothing
  could match `Box<T>`. The struct-literal counterpart of [col-literal-arg], and
  it ranks the same way: a written type argument or a concrete expectation
  decides, and the fields are evidence of **last resort**, so
  `Box<Str> { value: 7 }` is still reported. Each field's declared type is
  unified against its value's type, in field order, so a later field may widen
  an earlier binding exactly as a later argument does.
* [col-to-str] `to_str` of a collection is **the language's format, not the
  target's**, and both backends emit the same string: `[1, 2, 3]` for a
  list, `{1, 2, 3}` for a set, `{a: 1, b: 2}` for a map — the shape of the
  literal that would build it [col-literal]. Elements appear in the
  collection's own order ([col-insertion-order], or key order for the sorted
  pair).
  * Neither backend's native rendering is used, because they disagree with
    each other and with Salvo: Rust's `Debug` for a map quotes string keys
    and writes `:`, Kotlin's `toString` writes `a=1`. A set's and a map's
    `to_str` are Salvo (`core.set`, `core.map`, `core.sorted`; the maps' since
    2026-10-06, over a key `?to_str` and a value `?to_str`
    [implicit-same-name]), so each element renders by its own `to_str`.
* [col-map-entries] **`entries(map)` and `values(map)`** (2026-10-06) walk a
  map in insertion order without copying: `entries` emits `MapEntry<K, V>`
  (`{ key: proj K, value: proj V }`), a view struct as [col-enumerate]'s
  `Enumerated` is, and `values` emits `proj(map) V`. They step through the
  host's slots (`slot_count`, `key_at`, `value_at`, private to `core.map`),
  skipping removed ones.
* [col-map-eq] **`==` on a set or a map compares contents, in any order**
  (2026-10-06): `eq` for `Set`, `Map`, `SortedSet` and `SortedMap` is Salvo —
  the same size, and every element (key) of one found in the other by the
  container's identity, with equal values by the values' `eq` (a second
  implicit of the name, [implicit-same-name]).
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
* [col-equality] **Equality is a capability**: `a == b` is `eq(a, b)`, so a
  struct supports `==` and `!=` exactly when an `eq` for it is in scope —
  stamped with `: Eq<self> by auto` [obligation-by], or hand-written and
  declared on the type [fn-attached]. That **overturns** the 2026-09-12 decision that
  every struct compares structurally (user decision 2026-09-21): the operator
  now resolves like any call [op-equality], and a type with no `eq` says so.
  What survives from the old rule:
  * Both operands must be the **same base type** — comparing two different
    struct types is an error, not a constant `false` — and qualifiers are
    ignored on both sides (`Surname Person == Person` is fine): equality is
    about the data at the moment of the check, not about what is claimed of the
    handle. State, provenance and `Mut` alike.
  * A **fn-typed field bars the *structural* `eq`**: `Rc<dyn Fn>` has none on
    Rust and Kotlin would compare by reference, so no answer exists that both
    backends can give — `: Eq<self> by auto` fails at the copy for that field,
    naming it [comptime-instantiate]. It no
    longer bars the *struct*, which is the capability decision 6 opened: declare
    an `eq` that ignores the field and the type is comparable (and hashable,
    with a `hash` to match).
  * **Ordering and equality are separate axes still**, and both are now
    separate *functions*: `cmp` and `eq`.
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
* [col-hashed-ordered] A struct is a **key** when it *has the functions*: a
  `hash` and an `eq` for a `Set` element or a `Map` key, a `cmp` for a
  `SortedSet` element or a `SortedMap` key. `canbe hashed` and `canbe ordered`
  are **deleted** (user decision 2026-09-21) — the opt-in became the
  implementation, since there was never anything a declaration could opt into
  beyond having an ordering. `: Hashed<self> by auto` / `: Ordered<self> by
  auto` stamp the structural ones [obligation-by], and what a stamped body
  needs of the type is checked where the type is declared, so the error names
  the field rather than surfacing at a distant `Set<Point>`:
  * `core.auto`'s `hash` **refuses** a `canbe Mut` struct [comptime-refuse] — a
    value that can change while a collection holds it corrupts the collection's
    lookup or order, which is the classic silent-corruption bug made a compile
    error;
  * every field must itself have the capability, by ordinary resolution in the
    unrolled copy: `Int`, `Long`, `Str`, `Char` and `Bool` have all three;
    `Double`/`Float` have `eq` and neither `cmp` nor `hash` (Rust's `f64` is not
    `Eq`, `Hash` or `Ord`), so `: Eq<self> by auto` on a struct holding one
    works and `: Hashed<self> by auto` reports the field; a nested struct must
    have its own; a **`List` or a tuple** resolves `core.compare`'s Salvo
    `eq`/`cmp`/`hash` for it, which take the element types' own (an element
    struct's *declared* `cmp` is what orders a list of them, since 2026-10-06
    [implicit-recursive]). Ordering is lexicographic, with a shorter list
    that is a prefix comparing less.
  * Ordering of a struct is lexicographic **by field declaration order**,
    which makes field order semantically significant: the stamped `cmp` is one
    per-field `cmp` in that order. No backend derives or generates a host
    ordering or hash for a struct: a container of structs goes through the
    Salvo functions.
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
  * A literal also adopts the position's **identity** [cmp-carry]: `let s:
    Set<Str>(by_len, same_len) = {"ab"}` builds a set kept by those fns,
    empty or not (2026-10-05; before, the literal was built with the
    canonical identity and the slot pattern accepted it, so the value
    silently hashed the host's way).
  * **The checker resolves a set or map literal as that constructor call**
    (2026-10-06, ROADMAP §0j step 7): `{a, b}` is `set_of(a, b)` (or
    `mut_set_of` at a `Mut` position), `{k: v}` is `map_of((k, v))`, an empty
    `{}` at a `Map` is `map_of()`. It records the constructor at the
    literal's span, with its type arguments and its implicits filled from
    the identities the literal's type carries, so a backend renders an
    ordinary call (`literal_as_call`) and knows nothing of literals or
    identities.
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
    `platform handler` [platform-handler] in customer code.
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
* [for-elem-write] **A `for` body may write a field of its element** (user
  decision 2026-10-04): `for a in xs { a.dead = true }` over a `Mut List<Mut
  T>` changes the list's elements. Rust iterates `iter_mut()` when the body
  assigns through the element; Kotlin needed nothing.
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
      per loop shape — Kotlin cannot destructure a iterator's cast payload or a
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
  * [flow-or-not] **What a test proves holds only where the test did**:
    `a && b` narrows its then-branch by both, but `a || b` and `!a` narrow
    nothing in the branch they guard, and a name an `is` binds inside one is
    not in scope after it (the checker has always said so; the IR builder
    used to narrow the branch anyway). The right operand of `a || b` sees
    the *failure* of `a`.
  * [qual-lift] on a projection: inside `if h.f is ^Ok { … }` a further test
    of `h.f` reads the **lifted** view of the field (the wrapper peeled),
    not the stored union; the IR names that view with a narrowing binding
    until the branch closes.
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
* [struct-lit-shorthand] **A field may be written by its name alone** (user
  request 2026-10-05, noticeboard): `Person { name, age: 3 }` is `Person {
  name: name, age: 3 }`, the inverse of destructuring. Only after a type name:
  a bare `{ name }` is a set literal [col-literal], and `{ name: … }` still
  needs its colon. The value is an ordinary read of the variable, so it moves
  as `name: name` would.
* [struct-lit-infer] The struct literal's type annotation can be dropped
  when the expected type is known (`let p: Person = {name: ...}`).
  * A struct literal with no inferable type is a codegen error, never a
    guess.
* [type-no-cycle] **A struct may not contain itself** (the diagnostic the
  recursive-types plan owed, built 2026-09-25): a field whose type reaches the
  struct again *without indirecting* makes an infinitely large value. The error
  sits at the declaration, names the field that closes the cycle and the path it
  took, and names the remedy — indirect through a container.
  * Cycle edges are the ones that lay a value out **inline**: a field of struct
    type, a tuple element, a union arm, a nullable's inner type, an alias
    expansion, and a type argument a generic struct *stores* inline
    (`Wrapper<Looped>` where `Wrapper<T>` has `value: T`).
  * Not edges, because each indirects already: `List`, `Set`, `Map`, an array,
    a fn type, and a `proj` field. So `struct Tree { kids: List<Tree> }` is how
    a tree is written, a mutually recursive pair through containers is legal,
    and a generic whose parameter sits *inside* a container is too
    (`Bag<T> { items: List<T> }` with `N { b: Bag<N> }`) — the precision a
    name-based container test got wrong on the first attempt.
  * Why it is owed: Kotlin compiled such a struct (its fields are references)
    while Rust failed downstream with a raw E0072 and no Salvo diagnostic —
    a [backend-never-wrong] hole. The *feature* (a boxing rule that would make
    recursive types work) is separate and unscheduled; the refusal is honest
    either way. Two pre-existing tests used a recursive struct as an
    incidental fixture, which is how casually the hole was being relied on.
* [struct-mut] `struct Name canbe Mut { ... }` opts a struct into the `Mut`
  auto-qualifier; only `Mut Name` values may have fields assigned.
  * `Mut` is the only auto-qualifier.
  * Enforced at field-assignment sites since S1: assigning to a field of
    a struct value whose type is not `Mut`-qualified is an error (the
    `copy` intrinsic's identity lowering on Kotlin relies on non-`Mut`
    values really being immutable [copy-fn]). Arrays remain
    index-assignable without `Mut` (status quo; `copy` performs a real
    array copy).
* [field-canbe-mut] **`name: canbe Mut T` on a struct field**: the field is
  `Mut T` exactly when the struct value is `Mut`, and `T` otherwise, so a
  plain struct is immutable all the way down (user decision 2026-10-05,
  ROADMAP §0j step 6b; built 2026-10-05). A field written `Mut T` stays
  mutable inside a plain struct, as before.
  * One reading for every place a declared field type is used
    (`struct_field_ty` in the checker): a read `s.f`, a literal's expected
    field types (and their type-argument inference), destructuring.
  * Refused at the declaration: in a struct without `canbe Mut` (its value
    is never `Mut`), over a type whose declaration does not say `canbe Mut`
    (a generic `T` included), over a `T` already written `Mut`, and on a
    handler's state.
  * A **`Mut` literal** gives such a field a `Mut` value, and may not take it
    from a plain value: a spread source must be `Mut`, and a default (checked
    once, at the plain type) does not apply, so the field must be given.
    Otherwise the field would become mutable while a plain value shares it
    (Kotlin's `copy` of a plain value is the value itself [kt-copy]).
  * `copy` of a `Mut` struct copies the field as the `Mut T` it is there
    [copy-fn]; Rust's `clone` is deep already, Kotlin's copy is deep since
    the same change [kt-copy].
  * Backends: Rust erases `Mut`, so nothing changes; Kotlin stores both
    shapes in one property [kt-field-canbe-mut].
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
    **iterator struct** can declare that driving it uses it up
    ([iter-protocol], [once-fn]) — the alternative, inferring it from the
    presence of a `next`, would attach an obligation to someone's type on
    the strength of a method name.
* [qual-overload] **A qualifier name may be declared over several subject
  types**, and which one a use means is decided by the subject — the way a
  function overload is decided by its arguments (user decision 2026-09-13).
  std declares `NonEmpty` five times over, each **beside the container it
  claims** (2026-09-26): `of List<T>` in `core.list`, `of Set<T>` in
  `core.set`, `of Map<K, V>` in `core.map`, and `of SortedSet<T>` /
  `of SortedMap<K, V>` in `core.sorted`.
  * The four gathered in a `core.nonempty` of their own until 2026-09-26, for
    one reason: `min(NonEmpty SortedSet<T>)` has to delegate to the plain
    `min`, and `min@core.sorted(set)` re-picks the `NonEmpty` overload — a
    selector names a module, and within it the qualified argument still ranks
    first. `rename fn min_opt = min<T>(set: SortedSet<T>)` is what dissolved
    the module: it takes the plain overload out of the shared name *in that
    file only* (a rename is not importable [fn-rename]), which is the same
    escape `first(NonEmpty List<T>)` makes by delegating to `get`
    [col-of-nonempty].
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
    [qual-refn-ambiguous] — they are about different subjects.
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
    refinement-conflict rule [qual-refn-ambiguous] share
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
* [qual-value-arg] A qualifier's (or an `intrinsic type`'s) **value
  arguments live in a round-bracket block after the type generics**, at the
  declaration and at every use (user decision 2026-09-23):
  `qualifier Sorted<T>(?cmp: (T, T) -> Int) of List<T>`,
  `Heap(min_by_age) Mut List<Person>`, `SortedSet<Str>(by_len)`. The `?`
  prefix marks **implicit** entries (fn slots, resolved by name
  [implicit-resolve], and group spreads `?Ordered<T>`); identities are
  unprefixed; local references and constants will join the same block in
  later steps of the refinement-types sequence (ROADMAP.md).
  * **Type generics are all-or-none** at a use site: written in full or all
    inferred, never a partial list — so a reader never has to know a
    declaration's arity to tell which position an argument fills.
  * The lowered type keeps the one flat argument list ([types, then
    identities], unwritten types padded), so everything downstream of the
    lowering — binder collection [cmp-binder], erasure, the backends — is
    unchanged; diagnostics render the block form.
  * A `?` entry inside `<…>` is a parse error naming the block.
* [qual-depend] A qualifier may declare **value slots** — unprefixed
  `name: Type` entries in its block — making it a **dependent claim**: a
  fact about the subject's relationship to another value (user decisions
  2026-09-23; the refinement-types sequence, step 2). std's first is
  `core.map`'s `qualifier KeyOf<K, V>(map: Map<K, V>) of K`.
  * **The runtime tier is a dependent `qualifies`**: the subject first,
    then one parameter per value slot, each matching its slot's type —
    `KeyOf`'s is exactly `contains_key`. Tested with the **filled block**:
    `k is KeyOf(m)` lowers to `KeyOf.qualifies(k, m)` through the ordinary
    predicate path [is-qualifies], and `assert!(k is KeyOf(m))` narrows
    permanently [assert-narrow]. An unfilled test of a dependent qualifier
    is an error naming the filled form.
  * **A slot is filled with a place** — a variable or a field chain
    ([fate-link]'s provenance domain) — and the claim binds to the place's
    **fate roots**, so `let m2 = m` does not orphan it, and `KeyOf(m1)`
    and `KeyOf(m2)` are different facts (`Ty::ValueRef` carries the
    roots; a signature or annotation lowers a root-free template).
  * **Any mutation of the depended-on value strips the claim**, from every
    value holding it — the conservative direction: a `KeyOf` is a fact
    about the *map's* contents, and stripping is per-parameter only for
    claims about the parameter itself [deduce-syntax]. Reads keep it;
    mutating an unrelated value keeps it. The opt-back is the `preserve`
    entry (step 4 of the sequence). **Reassignment and `++`/`--` strip
    too** — the old value is gone, so a claim bound to it describes
    nothing (defect fixed 2026-09-24: an `Idx(xs)` claim held across
    `xs = [9]` kept resolving the total `get` — a checked out-of-bounds
    read at runtime).
  * One kind of slot per qualifier for now: fn slots [cmp-carry] or value
    slots, not both (nothing in std or the design's catalog mixes them).
  * **Signatures consume the claims** (step 3): a parameter type may fill
    a slot with a **sibling parameter** (`fn get<T>(list: List<T>, index:
    Idx(list) Int)`), and at each call the template is substituted with the
    roots the arguments bring. **And supply them** (2026-09-24, found
    building [col-update]): inside the declaring fn's own body the claim
    is *live* — its roots fill from the sibling parameters at declaration
    (`root_declared_claims`), so the body may hand its claimed parameter
    to a claim-demanding overload, and mutation or reassignment of the
    depended-on parameter strips it like any live claim. The signature
    callers match against stays a root-free template — so `get(xs, i)` demands a claim about *xs*,
    a claim about another list refuses (the plain overload takes the call),
    and an alias of the value still matches (roots, not names). Ranked
    against the unqualified overload by [fn-overload-rank], the
    `first(NonEmpty)` pattern.
  * **A total overload must shed the claim before delegating to its
    optional sibling** — with it attached, resolution re-picks the total
    overload and recurses; std's bodies re-derive a plain value
    (`index + 0`), the [col-of-nonempty] lesson in dependent form.
  * **An iterator mints claims per element** (step 5): its Yield clause's
    element may carry a dependent claim whose slot names the iterator's own
    borrowed field (`: Yield<self, Idx(self.items) Int>`), its `next`
    returns the established form (`-> Emitted (+Idx(p.items) Int) |
    Finished` — the clause and the `next` name one slot from two vantage
    points, matched by the field), and a `for` binds the claim to the
    **source's** roots — what `rev_indices(xs)` emits is an `Idx` of `xs`.
    Sound because the source cannot be mutated while the iterator lives
    [proj-infer]. The element keeps every qualifier of the `Emitted` arm
    beside the protocol tag itself.
  * **`+Q` is legal anywhere in a return type** ([deduce-reapply]'s
    establishment in arm position): `binary_search -> (+Idx(list) Int)?`
    mints inside the optional — trusted, so only in the qualifier's own
    file; the body's returns are checked without the established claims,
    and callers substitute their arguments' roots in.
  * Erased like everything about a qualifier [qual-erasure]: the places
    reach the backends only as the extra arguments of a lowered
    `qualifies` call.
* [qual-const] A qualifier's value slot may be filled with a **constant**
  — the third slot kind, after fn identities [cmp-carry] and places
  [qual-depend] (user decisions 2026-09-23; the refinement-types sequence,
  step 6): `core.range`'s `qualifier InRange(lo: Int, hi: Int) of Int`,
  used as `InRange(0, 65535) Int`. Compile-time known, so nothing tracks
  it and nothing can invalidate it; the dependent `qualifies` receives the
  constants as ordinary arguments (`n is InRange(0, 100)` lowers to
  `InRange.qualifies(n, 0, 100)`).
  * **Constants agree exactly or not at all**: `InRange(0, 100)` and
    `InRange(0, 255)` are different facts, and a parameter demanding one
    is not filled by the other. Range *containment* (`InRange(10, 20)`
    fitting an `InRange(0, 100)` position) would need the qualifier's own
    semantics — recorded in ROADMAP.md with literal establishment
    (`listen(8080)` proving itself), the two remainders of the step.
  * Parsed as a digit-named ref (the type language has no literal node —
    a recorded shortcut), lowered to `Ty::ConstInt`.
* [qual-preserve] A **`preserve` entry** opts a call back out of the
  conservative cross-value stripping [qual-depend]: `=> map: preserve
  KeyOf` says the call does not invalidate the named dependent claims
  other values hold about that parameter (user decisions 2026-09-23,
  spelled `preserve` — imperative like `defer`; "kept" already means *not
  consumed*, and the parameter is not the party holding the claim).
  * **In a refinement** [qual-refn]: the claim's owner states it for a
    call it does not own — std's `put` preserves `KeyOf` (writing never
    removes a key), `add` and `swap` preserve `Idx` (growth keeps every
    index valid; an exchange moves no boundary). Validated against the
    qualifier's **value slots** (the parameter is the value the claims
    depend on, not the claim's subject), and only a dependent qualifier
    may appear. Preservation cannot conflict [qual-refn-ambiguous] and
    merges across groups.
  * **In a fn's own clause**, alongside the parameter's ordinary entry
    (exempt from the once-rule — it is about *other* values' claims):
    **checked**, anywhere — every call in the body passing the parameter
    at a `Mut` position must itself preserve the claim, conditional calls
    included (the promise is unconditional). A bodiless declaration
    cannot promise it; the refn is the tool. Callers consume it exactly
    as they consume a refinement's.
  * Rendered in hover beside additions and removals ([qual-refn-docs]:
    `[list: +NonEmpty preserve Idx]`).
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
  **provenance** claim — about what the value *passed through*, which is
  content-independent. Two families use the one kind (user decision
  2026-09-23): **authority** (`Authenticated`, `EnvironmentId` — the value
  came through a checkpoint) and **protocol role** — std's tags `Ok`,
  `Err`, `Thrown` and `Emitted` are provenance (`ok(x)` is where an `Ok`
  comes from), which is what lets a tag survive mutation of the payload
  and stack with any content claim without `with`. The discriminator is
  content-dependence, not mint-onlyness: `Sorted` and `Heap` are mint-only
  *state* claims (contents; mutation must strip them), the tags are
  mint-only *provenance* (origin; it must not).
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
* [qual-ctor-fn] `fn f(...) -> +Q T { ... }` marks a constructor: every
  return point returns plain `T`; the qualifier is applied *by
  construction*, and callers see `Q T`. The `+` is the same establishment
  marker deductions use [deduce-reapply] — one spelling for one trust, in
  every position (user decision 2026-09-23, replacing the trailing
  `-> T as Q`, which itself replaced value-level `as` expressions and the
  `as` effect).
  * Union tagging goes through generic constructors
    (`fn ok<T>(value: T) -> +Ok T`).
* [qual-ctor-same-file] Constructor functions must be declared in the same
  file as their qualifier.
* [qual-result-tags] std ships the result tags in `core.result`:
  `provenance qualifier Ok<T> of T`, `provenance qualifier Err<T> of T` (provenance
  as of 2026-09-23 [qual-subject]), and their constructors
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
  * **`+Q` also appears in a function's own clause**, as of 2026-09-22
    [deduce-reapply] — the same word for the same trust, differing in who it is
    about: a refinement is the qualifier author's claim about *someone else's*
    call, while `=> p: +Q` is a claim about *this* body. Both are gated to the
    qualifier's own file, and neither proves anything: what a refinement is for
    is a call the author of `Q` does not own, which is why both exist.
  * **Applied after the callee's own list** at each call site
    [deduce-consume]: `add`'s exhaustive `=> list: Mut` drops `NonEmpty`,
    then the refinement puts it back. Only on the *kept* path — nothing is
    known about a moved parameter, and refining one is an error.
  * **Trusted**, like `-> +Q T` [qual-ctor-fn]: no `qualifies` call is
    emitted where a refinement applies, even for a predicate qualifier.
  * **State qualifiers only** [qual-subject]. Provenance cannot be
    invalidated, so there is nothing to re-establish; the compiler's own
    qualifiers carry representation choices and flow rules (`Mut` is not
    even erased), so a refinement may not hand one out. This is also what
    makes the whole feature invisible to the backends.
  * **A qualifier may only refine its own claim.** `NonEmpty` cannot say
    what a call does to `Sorted`. That is what makes [qual-refn-scope]'s
    opt-in honest, and it is why conflicts reduce to "two qualifiers that
    cannot co-apply" [qual-refn-ambiguous].
  * A refinement's qualifier must apply to the parameter's type
    [qual-of], and each entry must name a parameter of the resolved
    overload, once.
  * Effect members are **not** refinable (deferred, user decision
    2026-09-06): a member has no `FnKey` and naming one needs an
    effect-qualified form.
* [qual-refn-match] A refinement's parameter list picks **one** overload:
  it must repeat that overload's parameters — same names, same
  variadic/implicit flags, same types *under* their qualifiers, with type
  parameters matched by **position** (the refinement's own, preceded by the
  qualifier's), and every qualifier the declaration writes written too. Zero
  or several matches is an error at the refinement, so a typo cannot
  become a refinement that silently never fires. A name in the parameter
  list that is neither a type parameter nor a visible type is reported as
  such, naming the type-parameter remedy — a top-level `refn` has no
  qualifier to borrow `T` from.
* [deduce-gained] A plain (non-`+`) entry may name a qualifier the parameter
  does **not** declare, when the body *establishes* it — through a refinement,
  so the claim's owner said so [qual-refn]. The promise is then **checked**
  against the inferred facts like any plain entry, which is the whole difference
  from `+Q`: `+Q` is trusted and therefore legal only in the qualifier's own
  file, while this is verified and legal anywhere (user decision 2026-09-23,
  found on `std.heap`'s `push`).

  ```
  // `add` makes the list non-empty (core.list's refinement says so) and the
  // sift only swaps, so the heap comes back non-empty — reported, not claimed.
  export fn push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
  => heap: +Heap NonEmpty Mut, !elem { … }
  ```

  * **The caller learns it**, which is the point: after `push(h, 3)` the heap is
    `NonEmpty`, so `pop` resolves to the overload that answers an element.
  * **Only a written clause reports a gain.** An inferred one does not, even
    when the body establishes something: handing a caller a claim is a
    signature's job to say out loud.
  * **A bodiless declaration cannot** (`intrinsic fn`, an effect member): there
    is nothing to check the report against, and the diagnostic says so.
  * The two failures read differently on purpose: "the body may remove it" for a
    qualifier the parameter *has*, and "not declared on parameter … and nothing
    in the body establishes it" for one it never had.
* [qual-refn-narrow] A refinement may write a **narrower** parameter than the
  declaration it refines, and the extra qualifiers are a **precondition**: what
  it states applies only where the argument already carries them (user
  correction 2026-09-23).

  ```
  // Swapping two elements of an *already* non-empty list leaves it non-empty.
  refn swap(list: NonEmpty Mut List<T>, i: Int, j: Int) => list: +NonEmpty
  ```

  This is the difference between a claim a call **establishes** and one it
  merely **keeps**, and it is not decoration: the unconditional form of the
  refinement above would say that swapping *makes* a list non-empty, which is
  false for an empty one. `add` establishes (`refn add(list: Mut List<T>, elem:
  T) => list: +NonEmpty`); `swap` keeps.
  * Preconditions are per parameter, and two refinements of one parameter that
    require different things are two independent groups: each applies where its
    own precondition holds. Disagreement, however, is judged **across** groups
    as well as within one [qual-refn-ambiguous]: two statements that apply to the
    same call cannot establish claims that one value could not carry, whatever
    their preconditions were.
  * A **kept** claim survives a **conditional** call, an established one does
    not: if the call may not have happened, the caller's own claim is still
    whatever it was, but nothing new has been established. (Before this rule the
    deduction pass suppressed *both* inside a branch, which is why
    `std.heap`'s sift-down had to swap unconditionally.)
  * What a signature *reads* as, and what the deduction pass uses for an
    unconditional promise, is the precondition-free group — so a reader is never
    told a conditional fact as though it always held [qual-refn-docs].
* [qual-refn-scope] **A refinement lives in the qualifier whose claim it is
  about** — always (user decision 2026-09-26). It applies wherever that
  qualifier is in scope and nowhere else: the user opts into the refinements by
  opting into the qualifier (user decision 2026-09-06).
  * A **top-level** `refn` is refused, with a diagnostic naming the move. It
    existed to *reconcile* two qualifiers that disagreed (it replaced their
    refinements for the parameters it named), and disagreement is now refused at
    the call and settled by naming a place [qual-refn-ambiguous]
    [qual-refn-at] — so the form had no job left, and having one refinement in
    two possible places was the cost.
  * A **constructive** qualifier may therefore have a body: what makes a
    qualifier *predicate* is holding a `qualifies` [qual-predicate], not holding
    braces. A body of refinements alone is the mint-only shape — `Sorted`'s
    insert is the case that forced it: the claim cannot be tested, and still has
    something to say about a function that keeps it.
* [qual-refn-ambiguous] When the refinements applying to one (callee,
  parameter) **disagree**, the call is an **error** (user decision 2026-09-26):
  the compiler refuses to choose, and the diagnostic names the `f@place` that
  picks each side. Two refinements disagree when their additions could not have
  been written together [qual-with]; an addition and a removal of the same
  qualifier disagree outright.
  * Disagreement is judged across **every** applicable group, not only within
    one: two refinements with different preconditions [qual-refn-narrow] are
    separate groups, and before this rule both were applied in order and the
    second addition was silently dropped for being incompatible with the first.
    That is the shape that motivated the rule — std says `add` establishes
    `NonEmpty`, your own qualifier says it keeps `NE`, and the caller ended up
    with whichever group ran first.
  * **Two refinements made by the *same* place** are an error at the
    refinements themselves, not at the call: `f@place` picks a place, so a
    place that disagrees with itself leaves the caller no way to choose. One of
    them has to go, or the qualifiers have to declare `with` each other.
  * Nothing is applied in either case, so no claim is invented from a choice
    the program did not make.
* [qual-refn-at] **`f@place(args)` picks the place whose statement is meant**,
  and only that place's refinements apply — rebuilt from its own statements, so
  a selector settles a disagreement rather than inheriting it. A place that
  refines a callee is a legal selector even when it declares no overload of it
  [fn-overload-at]. The unpicked statement does not apply, which is the point:
  `add@core.list(xs, 1)` establishes `NonEmpty` and leaves `NE` behind.
* [qual-refn-infer] Refinements reach **inferred** deductions
  [deduce-infer], so the fact survives one frame outward: a fn whose
  parameter declares the qualifier and whose body makes a refined call
  may promise it back, and a *written* list promising it validates against
  the same body facts. Two limits keep this sound:
  * An addition contributes only for a qualifier the parameter itself
    **declares** — it can cancel a removal, never invent a claim. The same
    limit holds for a fn's own `+Q` [deduce-reapply]: re-establishing is not
    adding.
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
* [op-wrap] **Integer arithmetic wraps** (user decision 2026-09-23, A-6;
  built 2026-10-05, ROADMAP §0j step 2): `Int`/`Long` `+`, `-`, `*`, unary `-`
  and `++`/`--` keep the low 32/64 bits on both backends — the JVM's
  behaviour, which Rust renders through `wrapping_*` [rs-op-wrap]. Division
  and remainder are not covered (§10's division row). `checked_*` /
  `saturating_*` functions for the cases that care are recorded, not built.
* [op-bits] **Bitwise operations are `bit_` functions** (user decision
  2026-10-05), not operators — `&`, `|`, `^` and `>>` already mean other
  things: `bit_and`, `bit_or`, `bit_xor`, `bit_not`, `bit_shl`, `bit_shr`
  (arithmetic) and `bit_ushr` (logical), each an `intrinsic fn` for `Int` and
  `Long` in `core.basic`. A shift count is taken modulo the width (the JVM's
  rule), so both backends agree for every count, negative ones included.
  `mix_hash` is ordinary Salvo over [op-wrap] since.
* [op-convert] `core.basic` declares the explicit conversions the
  operators point at: `to_int`, `to_long`, `to_float`, `to_double`, one
  overload per source width [intrinsic-fn]. Truncating conversions
  truncate toward zero and **saturate** at the target's bounds
  identically on both backends; `to_int(Long)` keeps the low 32 bits
  (Kotlin `toX()` ≡ Rust `as`, verified pairwise).
* [op-order] **Ordering is `cmp`**: `a < b` is `cmp(a, b) < 0`, and likewise
  for `<=`, `>` and `>=` (user decision 2026-09-21, superseding the
  2026-09-14 surface). So ordering works wherever a `cmp` is in scope:
  * **Numeric operands keep the native fast path**, widened per [op-promote] —
    the canonical implementation for a primitive *is* the host's operator, so
    emitting the operator is emitting the implementation. `Double` is orderable
    at the operator (both backends agree on IEEE partial comparison, `NaN`
    answering `false`) while a *sorted container* of them stays refused (no
    total order) — the same deliberate difference as before, now expressed as
    "`cmp(Double, Double)` does not exist, but `<` on numerics does not need
    it".
  * **Everything else resolves `cmp`** [implicit-resolve]: `Str` now orders (by
    code point, on both backends, which is what `cmp(Str, Str)` promises and
    the JVM's `<` does not), a struct orders when it has a `cmp`, and a tuple,
    a container or a fn value orders when someone declares one. With none in
    scope the operator is an error naming the remedy — a `cmp` inside the
    type's body, or `: Ordered<self> by auto`.
  * **At a generic `T`** the only candidate is an enclosing implicit parameter
    [implicit-forward], so a comparison in generic code publishes the
    capability in the signature (`?Ordered<T>`). Comparing an unconstrained `T`
    used to compile silently and emit `a < b` on a boundless generic; that hole
    is closed.
* [op-equality] **Equality is `eq`**: `a == b` is `eq(a, b)` and `a != b` its
  negation, resolved exactly as ordering resolves `cmp` (user decision
  2026-09-21). Equality is therefore **opt-in** for a type of your own
  [col-equality].
  * **The intrinsic types keep the native operator**: `eq(Int, Int)`,
    `eq(Str, Str)` and their siblings *are* `==` on both hosts, so resolving
    them would buy nothing and cost every comparison an indirection. Ordering
    is deliberately not on that path, because `Str` must compare by code point
    where the JVM's `<` compares code units [kt-ordered].
  * A **possibly-absent** operand is still refused [op-no-none], and numeric
    widths still mix [op-promote].
  * The checker records which function each comparison resolved to, and both
    emitters read it: the operator and the call cannot disagree about what
    `==` means.
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
    [struct-lit-condition] Inside parentheses (call arguments, a
    parenthesized expression) it is enabled again, since a `{` there cannot
    open the block: `for n in notices(Path { text: t }, s) { … }` (fixed
    2026-10-05).
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
  * **A borrowed optional narrows like the optional** (2026-10-06): the
    total `get` over a `List<P?>` answers `proj(xs) P?`, a view of the whole
    optional, and `s is P p` binds `p: proj(xs) P` — each side keeps the
    `proj`. It used to bind `p` at `P?` (a field read on it was refused).
* [is-not] `x !is Q` is **sugar for `!(x is Q)`** (user decision 2026-09-22,
  the heap plan's item 5): the parser builds one `Expr::Is` under a `Not`, so
  narrowing, `when` heads and [is-narrow-guard]'s fall-through all reach it
  without knowing the spelling exists — `analyze_cond`'s `Not` arm already swaps
  the two narrow sets, which is what makes `if x !is Q { return }` narrow the
  rest of the block.
  * **It had to be a parser rule, because the spelling was already legal and
    meant something else.** `!` is the assert postfix [type-nullable], so
    `s !is Str` parsed as `(s!) is Str` — asserting the value present and then
    testing it, which type-checks and reads as the *opposite* of the intent. So
    the postfix tier now leaves a `!` alone when `is` follows it; `(s!) is Str`
    is still writable with the parentheses.
  * **No binding and no `^`**: a negated test tells you nothing on the branch it
    guards, so there is no value to name (`!is Q name`) and none to widen
    (`!is ^Q` [qual-lift]). Both are errors naming the positive form.
  * Everything it composes with comes free: `&&`/`||`, a `when` condition head,
    and the overload routing after the guard [fn-overload-rank].
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
* [is-and-chain] **An `is` binding is in scope in the rest of its `&&`
  chain**: `if b is Long known && known == x { … }`. Both backends bind it
  inside the `&&` once the test has passed (Rust `lhs && { let known = …;
  rhs }`, Kotlin `lhs && run { … }`), and again for the body as before
  (fixed 2026-10-04; it was refused by both host compilers).
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
    and the lifted union lives in the name. Built end to end (2026-09-21): the
    bound value spans fewer arms than its storage, so it is produced by the
    arm mapping [rewrap] rather than by a payload read. **Without** a binding
    it stays refused for the original reason: one re-read cannot stand for two
    wrapper positions.
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
* [for-iter] `for x in e` drives `e` when it is an iterator [iter-protocol] and
  otherwise iterates `iter(e)` implicitly [iter-mint]; missing or ambiguous
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
  must declare its deduction clause (it may not declare effects at all
  [effect-member-no-effects]).
  * [effect-member-none] An effect member with no return type returns
    `None`, as a fn does (user decision 2026-10-04; it had to write `-> None`
    until then): `None` is what an absent return type means everywhere else,
    so leaving it off guesses nothing. Inference from an absent body is a guess,
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
  declaration form. Each error names the surviving forms — write the body,
  or declare it `platform fn` for the host to implement [platform-fn]; give
  the type a definition (`type N = ...`), or declare it `intrinsic`.
  `intrinsic` and `platform` declarations parse through their own modifier
  branches, so they are the only bodiless `fn`/`type` forms left; only std
  may write `intrinsic` [intrinsic-std-only].
* [platform-fn] **`platform fn name(…) -> T`** declares a top-level fn the
  host implements in the target language (user decision 2026-10-01, the ABI decisions
  D1; the modifier replaced "a bodiless fn" as the marker, so every interop
  point is one searchable word). It is a signature only: a body is a parse
  error, as is `threadsafe` (a claim about a handler's class). `export
  platform fn` exports it.
  * Its implementation is a function with the **real name** in the module's
    implementation file under the platform root [platform-tree]
    (`salvo.platform.<m>.shout` / `crate::platform_<m>::shout`). The program
    calls a generated **wrapper**, `shoutPlatform` (Kotlin) /
    `shout_platform` (Rust), which calls the implementation (user decision
    2026-10-01, the ABI decisions); the wrapper is where boundary validation goes. A
    program reaching a platform fn whose implementation file is missing is a
    codegen error naming `salvo platform generate`.
  * No implicit parameters, no effects beyond `[]`: the host writes a
    function that performs no Salvo effect. Type parameters are allowed and
    opaque to the host [platform-generic].
  * **Two platform fns may not overload each other** (user decision
    2026-10-01, ABI D5): each gets its own namespace at the boundary. A
    platform fn may share its name with ordinary fns.
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
* [fn-overload-scope] **Scope says which functions are *visible*, never which
  one a call means** (user decision 2026-09-26, replacing the scope-precedence
  rule of 2026-09-07). Functions arrive from `core`, from this file's imports
  and from this module, and **every** one that fits the arguments competes on
  signature alone [fn-overload-rank]; no single most specific candidate is an
  error naming the places [fn-overload-ambiguous].
  * So a module declaring its own `size(List<T>)` does **not** quietly take
    over `size(xs)` in its own file: both fit, neither is more specific, and
    the call writes `size@mymodule(xs)` or `size@core.list(xs)`. The rule it
    replaces made which one ran depend on a ladder the reader had to know std's
    surface to predict — and the previous compromise (pick the nearer scope,
    warn when a *more specific* signature was discarded) still chose, which is
    what the language stopped doing.
  * `rename fn` [fn-rename] and `import … as` are the remedies for a name used
    often in one scope: both take one declaration out of the shared name, so
    the calls need no selector at all. That is the intended shape for a module
    that means to work with a shadowing overload throughout.
  * **One declaration is one candidate.** A fn reachable by two routes — an
    implicitly available name that is also a member of the file's own module
    (std's `test` inside an annex, `core.list` inside `core.list`) — is not an
    ambiguity: the pool is deduped by declaration before ranking.
  * The rungs above the overload set are unchanged, and they *shadow* rather
    than compete: a fn-typed local, parameter or implicit **is** the function
    the caller chose, an effect member takes the name before any fn does
    [effect-member-overload], and a rename introduces a fresh name. `@` is the
    way to reach a fn a local shadows [fn-overload-at].
* [fn-overload-rank] **Then the most specific signature**, compared **per
  argument slot** (`types::spec_cmp`, `rank_cmp`):
  1. a **type variable** says the least, structurally (`List<Int>` beats
     `List<T>`) — **whatever its qualifiers**: `Set<T>` beats `Mut It` for a
     `Mut Set<Str>` argument, since a qualifier on a type variable is a
     permission the fn asks for, not knowledge of the type (user decision
     2026-10-04; it was unrankable under rule 3 until then);
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
* [implicit-fit] **A candidate whose implicits nothing can fill does not
  fit** (user decision 2026-10-04): among several candidates, one with an
  implicit parameter that no named argument, forwarded implicit or visible fn
  of that name accepting the wanted parameters can fill is dropped before
  ranking. So std's generic `count<It, T>(it: Mut It, ?Yield<It, T>)` neither
  outranks nor blocks a program's own `count(NonEmpty List<T>)`; when every
  candidate drops out, the error is "no matching overload", naming the
  implicit. Checked by the parameters' shape only, since the answer type may
  not be known at ranking; a lone candidate keeps its own diagnostic about
  the implicit.
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
  * The module may be one that **refines** the callee rather than declaring it
    [qual-refn-at]: `add@mymodule(xs, 1)` names this module's statement about
    `core.list`'s `add`. Then every declaration still competes and the selector
    narrows the refinements instead.
  * **A selector that changes nothing is a warning** (user decision
    2026-09-26): if the call resolves to the same declaration without it, and
    no refinement of that callee disagrees in this file, the `@place` is noise —
    and noise in a disambiguation spelling reads as evidence that something is
    ambiguous. Still a correct program, so a warning, never an error.
  * Naming a module with no *fitting* overload is an error listing the
    modules that have one — never a silent fallback.
  * It is the way out of a **shadowed** name: a local of the same name hides
    every function, and `@` is what reaches one anyway.
  * On a renamed name it is an error: a rename already names one
    declaration.
  * A capitalized name that is a **type** is the third sibling of this family:
    a type selector, naming a fn declared on it [fn-attached]. Effects and types share the
    capitalized namespace [name-casing] and one name cannot be both in scope,
    so the reading is decided by what the name declares — not by new grammar.
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
  diagnostic names a `platform handler` [platform-handler] as the way
  to reach a target-language method. The checker records each call it read
  this way (`Checked::dot_calls`); a backend normalizes exactly those and
  never guesses from the name (built 2026-10-06).
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
  through a declared accessor (an `intrinsic fn`, or an effect member a
  `platform handler` implements, in customer code).
* [index-resolve] `[]` subscripts arrays only. Other collections expose
  element access as declared functions (std's `get(list, index)`), and
  tuples use constant positions ([expr-tuple-index]).
* [iter-resolve] A `for` subject must be an array, a **step call**
  [iter-step-call], an **iterator struct** — a type declaring `: Yield<self, T>`
  [iter-protocol] [group-obligation] — or a **source**: a value some declared
  `iter` overload accepts (the implicit `iter(subject)` call), a type declaring
  `: Iter<self, T>` [iter-group], or a generic under a `?Iter<C, T>` spread;
  anything else is an error. When the subject has a protocol-shaped `next` but
  no declaration, the error names the `: Yield<self, T>` remedy.
  * Resolution order: a step call, arrays natively, then the declared iterator
    struct [iter-protocol], then `iter` [iter-mint]. The struct comes before
    `iter` because a type with both is *already* a position in a sequence, so
    minting a second iterator from it would be wrong — and declaring both is
    refused at the struct [iter-group].
  * **Both lookups match the subject by declaration, not by type name** (fixed
    2026-09-25): a `Ty::Named` carries no module, so two same-named structs in
    two modules are one type as far as unification is concerned — a user
    `Range` beside `core.range`'s drove the *other* module's `next`, or minted
    with the other module's `iter`, and the emitted program was rejected by
    both target compilers (E0308 / an argument type mismatch). Wrong output
    rather than a diagnostic, so [backend-never-wrong]-grade. Each side's name
    is resolved in *its own* file's scope and the declarations compared; a name
    that differs, or a subject that is not a plain struct, stays `unify`'s
    business.
* [iter-generic-drive] A `for` over a **type parameter** drives it when the
  enclosing fn has a protocol-shaped `?Yield<It, T>` spread for it
  [implicit-group] (user decision 2026-09-09): the position *is* the declaration
  — it says "this call supplies a `next` for `It`" — so the loop calls that
  implicit parameter and takes the element type from its result. This is what
  lets std's own combinators be written with `for`, and any combinator of one's
  own with them. A type parameter that is a **source** under a `?Iter<C, T>`
  spread is minted with the `iter` implicit first and driven with the `next`
  beside it [iter-group].
  * **By name, not by shape**: the implicit must be called `next`, which is what
    `for` drives everywhere else [iter-protocol]; and it must take its state as
    `Mut It`, for the same reason a declared `next` does (the same diagnostic
    fires when it does not).
  * **The element is owned.** `next` hands the element over by value, so the
    loop binding is *not* a projection of the iterator and may be moved on — which
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
  * An iterator named by a variable or parameter is advanced in place: the
    position the loop reaches is what the owner sees next, the loop counts
    as a **mutation** rather than a move, driving an exhausted iterator again
    is legal (zero iterations), and the discharge stays the owner's —
    a linear iterator is bound with `let`, driven, and explicitly discharged
    after the loop and before early exits (the ordinary all-paths
    analysis enforces it) [linear-group].
  * Only a **temporary** subject (a minted iterator, a call result) is
    consumed by the loop — and a *linear* temporary is refused there
    ("a `for` cannot consume a linear iterator"): the loop never discharges
    what it drives, so an owned linear iterator has to be a named place.
  * The in-place rule exists because the two backends disagreed without
    it: Rust bound the subject into a local — a *clone*, for a kept
    parameter's `&mut` — so the caller never saw the position the loop
    reached, while Kotlin aliased it and did [backend-parity].
  * [linear-generics] A **generic** iterator under `canbe linear` follows the
    same rules: drive it in place and let the owner discharge, or — for a
    fn that owns the iterator — take a **consuming callback**
    (`end: (x: It) -> None` with `=>[end] !x`) and hand the iterator to it
    after the loop; callers pass the type's own discharger for a linear
    pass and std's `drop` for a plain one [linear-discard]. (This
    replaces the deleted `?Linear<It>` spread.)
* [iter-mint] `iter` converts a **container** into a fresh iterator
  (`fn iter<T>(list: List<T>) [] -> Mut ListYield<T> => list`), and that is
  the whole of container iteration: std declares an iterator struct plus a
  `next` per intrinsic container, so the language has no container protocol of
  its own (user decision 2026-09-08, roadmap R5). A type of one's own declares
  the same through `: Iter<self, T>` [iter-group].
  * The container is **borrowed by** the iterator (a `proj` field
    [proj-field]; user decision 2026-09-11 — until then it was moved in):
    `iter(xs)` keeps `xs` usable and links the iterator to it, so walking the
    same container twice is `iter(xs)` twice, and mutating `xs` while an iterator
    over it lives is refused [proj-infer].
  * A `for` over a container is lowered as *mint then drive*: the checker
    records the `iter` to call beside the `next` to drive, and **both emitters
    call it** — until 2026-09-09 the record was read by nobody, so a `for` over a
    container of one's own emitted a drive of the container itself, which the
    target compiler rejected (found while building [iter-fn], whose generated
    `iter` walks the same path).
  * **`for` over a *generic* source** mints through the `iter` implicit a
    `?Iter<C, T>` spread brought in, recorded as `PassMember::Implicit("iter")`
    [iter-group].
* [iter-for-native] A `for` over an **array** or an **`iterable platform
  type`** [platform-iterable] (`List`, `Str`, `Bytes`, `Deque`, `Set`, `Map`,
  the sorted pair) records no driver at all: the backends iterate
  natively, which neither allocates a Salvo iterator nor consumes the
  subject. The language gets no special case (the rule is a declaration, not
  a name list); the fast path is the emitters'.
* [iter-protocol] The pull iteration protocol is declared in std
  (`std/core/iterator.sv`), not built into the compiler: an **iterator
  struct** is a value some `next` accepts, and `next` reports
  `Emitted T | Finished` (user decision 2026-09-07; the names were
  `Next`/`Stopped` in the design; "pass" was the term until 2026-09-27, when
  the user renamed it — a value of an iterator struct is **an iterator**). This
  is the manual half of the iterator story — `zip`, `merge`, anything reading
  two sources at once — which a step function expresses directly; `iter fn`
  is the sugar [iter-fn] and `Iter` the source side [iter-group].
  * `Emitted` is a *provenance qualifier* (`provenance qualifier Emitted<T>
    of T` [qual-subject]) so the element
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
    state is `Mut` because advancing an iterator mutates its position.
  * Only the exact `Emitted T | Finished` shape is a driver; a `next` of
    any other shape is an ordinary function.
  * **`for` reads the declaration** — the `: Yield<self, T>` clause is the one
    fact that makes a value an iterator, and the element type is the clause's
    argument. The overload scan only resolves *which* `next` (and the arm
    identity); when the obligation is declared but unsatisfied, the error
    has already landed at the struct and the loop stays lenient, answering
    the declared element type [type-unknown-lenient]. A matching `next`
    without the clause is not an iterator ([iter-resolve] names the remedy) —
    the tie is declared, never inferred from a method name. A step under
    another name is driven by naming the call [iter-step-call].
  * Driving consumes the subject: it is moved into the loop, exactly as the
    `once` passes it replaced were. Drive-in-place (`Mut` borrow — "a
    second drive continues") is recorded as the eventual semantics and
    deferred with `once`-on-producers\' deletion (R5).
  * `next` takes its state as `Mut St`: advancing an iterator mutates its
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
    local — driving consumes an iterator, so nothing else is looking at
    it — and each turn calls `next` on a mutable place.
    * Rust: `while let Union2::U1(mut n) = next(&mut __loop1_pass) {`. A
      `while let` re-evaluates its condition per turn, so `Finished` needs
      no arm of its own.
    * Kotlin: `while (true)` plus `if (step !is Union2.U1<…>) { break }`, since
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
* [seq-lazy] **Lazy adaptors are named by a present participle** (user
  decisions 2026-10-03/04): `mapping(it, f)`, `filtering(it, keep)`,
  `taking(it, n)`, `taking_while(it, keep)`, `skipping(it, n)` and
  `skipping_while(it, skip)` answer an iterator over another, while `map` and
  `filter` stay eager and answer lists. Nothing runs until the iterator is
  driven, so an endless source costs nothing and a `break` stops it. Each is
  a struct (`Mapping<It, T, U>`, `Taking<It, T>`, …) holding the source
  iterator and the source's `next`, kept as a fn field from the `?Yield<It,
  T>` implicit, plus its own state; its `next` pulls one element at a time.
  `to_list(it)` drives one to its end as a list (a view of a borrowing
  source's elements, as `filter`'s is); a container's own `to_list` is more
  specific and wins [fn-overload-rank]. There is no `count(it)`: see ROADMAP
  0c item 17. Rust: [rs-lazy-adaptor].
* [seq-iterator] std's sequence functions (`map`, `filter`, `reduce`) take their
  subject as an **iterator** and reach its `next` through a `?Yield<It, T>`
  spread [implicit-group] — the group std declares for iteration — so any
  iterator is a subject: the one an `iter` hands back for a `List<T>`, an array
  or a `Str`, the one an `iter fn` mints, or an iterator struct of one's own. A
  container is iterated by *writing* its `iter` (`map(iter(xs), f)`), which is
  what keeps the inference ordinary: `It` is bound by an argument, so nothing
  depends on feeding one implicit's resolution into another (user decision
  2026-09-09, replacing `?Iterable`). There is no iterator *type*; the source
  side of the protocol is `params Iter<C, T>` [iter-group], which a program's
  own container-shaped combinator spreads.
  * **std relies on the iterator and never on an `iter`** (user decision
    2026-09-10), and the reason is stronger than the inference one: a source is
    not guaranteed to *have* a container behind it. An `iter fn`'s iterator, a
    composed iterator someone wrote by hand, a hand-written `zip` — for each of those the
    iterator is all there is, so a std function that asked for an `iter` would
    exclude them by construction. A *program* may still write a
    container-shaped combinator (`?iter` as an implicit, whose result determines
    the iterator type [implicit-infer]); std may not.
    * The `List` fast paths are not an exception: they are overloads on a
      concrete intrinsic type, lowered to the target's own collection
      operations, and they ask for no `iter` [fn-overload-rank].
  * **Eager, with one named variant** (user decisions 2026-09-08, 2026-09-10).
    `map`/`filter`/`reduce` return `Mut List<U>`; the default is the one that
    surprises least, and chaining works because a list has an `iter`.
    * **Nothing in std is lazy.** `map_lazy`/`filter_lazy` — composed iterators
      that computed as they were driven — were **removed 2026-09-10** (user
      decision): laziness as a data structure couples the data to the functions
      over it, and the direction to try instead is composing *functions*,
      `iter fn`s included, into pipelines that mint an iterator from data supplied
      separately. Reconsidered after concurrency lands; see ROADMAP.md. A
      composed iterator remains ordinary code for a program to write — an iterator is
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
  * A spread may only supply the **variadic tail** (2026-09-23): a call whose
    spread would land in a *required* parameter does not match, and the
    diagnostic says why — a spread's length is not known statically, so it
    cannot stand in for a parameter that must be there. Before the rule, such a
    call unified the *array* with the parameter's type, which silently bound a
    type parameter to it: `list_of(...xs)` against
    `list_of(first: T, ...rest: T[])` built a `List<Int[]>` and claimed
    `NonEmpty` for a possibly empty spread [backend-never-wrong]
    [col-of-nonempty].
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
    **kept**: it belongs to whoever supplied it.
  * [implicit-resolve-body] **It joins resolution by argument types** (user
    decision 2026-10-05, ROADMAP §0j 6i, option A; it used to be said to
    shadow the fns of its name, which the checker never did and the
    emitters did): a call to its name goes through the implicit when the
    arguments fit it — it *is* one of those functions, chosen by the caller —
    and to the visible fns of the name otherwise. So `cmp(a, b)` at `T` and
    `cmp(x.n, y.n)` at `Int` sit in one body, which a stamp at a generic
    struct needs [comptime-generic].
    * The arguments are typed once: a call that falls through hands their
      types to ordinary resolution. A call with a lambda or spread argument
      stays with the implicit.
    * The checker records which (`local_calls`); the emitters render through
      the parameter only for a recorded call. Rust qualifies a shadowed
      global as `crate::…`; Kotlin's own resolution passes an inapplicable
      local `invoke` by.
    * An ordinary fn-typed local still outranks every declaration
      [call-resolve], and a call through one now **checks its arguments**
      (by base shape) and its arity: `f(1)` with `f: (Str) -> Int` was
      accepted until 2026-10-05 and refused by the target compiler.
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
* [implicit-recursive] **Resolution is recursive** (user decisions
  2026-09-28, §6; built 2026-10-06): a candidate for an implicit position may
  have implicits of its own — `eq<T>(a: List<T>, b: List<T>, ?eq: (T, T) ->
  Bool)` — and those are filled in turn, by the same rules, with the
  candidate's generics bound as its fit bound them. The fill is recorded as
  `ImplicitArg::Resolved::nested`, and both backends' adapters pass it after
  the fn's explicit arguments.
  * A candidate whose implicits cannot be filled is a *near miss*: another
    candidate that resolves fully wins; if none does, the error names the
    chain (`eq for List<Foo> needs implicits of its own, and no eq …`).
  * **Depth cap 8** (`resolution of eq for … nests more than 8 levels deep;
    pass eq = … explicitly`) and a chain that needs itself is refused at once
    as a cycle.
  * std's `List` and tuple `eq`/`cmp`/`hash`/`to_str` are Salvo on it
    (`core.compare`, `core.basic`); the host's structural comparisons are
    gone. A tuple takes one implicit per part, of one name at different types
    [implicit-same-name].
  * Not yet: the carried identity is still a flat name [cmp-carry] — a
    `Set<List<Person>>` records `hash@List`, and the element's `hash` is
    resolved where the set is *used* rather than carried in the type; and the
    typed override.
* [implicit-same-name] **Two implicits of one name at different types are
  two parameters** (user decision 2026-09-28, §16's "riding along"; built
  2026-10-06): `fn to_str<K, V>(map: Map<K, V>(?hash, ?eq), ?to_str: (k: K) ->
  Str, ?to_str: (v: V) -> Str)`. Positions of one name *and* one shape (the
  parameter and result types, whatever the parameters are called) still merge
  into one [implicit-group].
  * **Inside the body**, a call of the name goes through the one whose
    parameters the arguments fit, and so do interpolation, a comparison
    operator and a forwarded fill; arguments that fit more than one are an
    error naming both. The checker binds the second and later as
    `to_str__1`, … (`ImplicitParam::local`) and records the one a call took
    (`local_call_names`), which is the name the backends declare and call.
  * **At a call site** each is filled on its own — a binder from the
    argument's type, the rest by name and type. The bare override `to_str = f`
    cannot say which it fills and is refused; the typed spelling (`to_str:
    (V) -> Str = f`) is decided and not built.
* [implicit-group] `params Field<T> { fn add(a: T, b: T) -> T ... }` declares
  a named bundle, spread into a signature as `?Field<T>`. It is a
  *declaration-side* shorthand only: the members become implicit parameters
  in their own right, and the group has **no binder** (user decision
  2026-09-05 — the binder referenced nothing, prevented no collision, and
  made the caller name it to override one member).
  * **A group is a convenience, not a contract** (user decision 2026-09-26): a
    caller may fill any subset of its members, and a **mixed fill** — one
    member written, the rest left to resolution — is honoured rather than
    refused. What a group *may* state is that some of its members go together
    [implicit-with].
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
    position — which is every iterator's `next`.)
  * The expansion order — written implicits first, then each group's members
    in declaration order — is published by the checker as the one ordered
    list both the callee's parameters and the caller's arguments follow.
  * **Two spreads asking for the same position ask for one parameter** (user
    decision 2026-09-22): `?Eq<T>` beside `?Hashed<T>` brings **one** `eq`, since
    `Hashed` carries the `eq` its `hash` is confirmed by [cmp-groups]. Merging is
    by name *and* type — it is one function position, named once — and a binder
    among the merged occurrences makes the merged position a binder
    [cmp-binder].
  * Two implicits of the same name at **different types** are an error: with no
    binder nothing tells them apart, and [var-no-shadow] would refuse them in
    the body. The message names both types; the remedy is to write the clashing
    ones out individually.
  * **A spread resolves every member, used or not** (user decision 2026-09-22):
    `?Hashed<T>` asks the call site for exactly what the group declares, so a `T`
    with a `hash` and no `eq` is the ordinary missing-implicit error naming `eq`.
    Asking for less is a narrower group, or the members written individually —
    which is what keeps `?cmp: (T, T) -> Int` a spelling worth having.
* [group-obligation] A `params` group may be stated as an **obligation** on
  a struct declaration: `linear struct Lines : Yield<self, Str> canbe Mut { … }`
  — a `:` clause between the generics and `canbe`, comma-separated, each
  entry a group name with type arguments (user decisions 2026-09-08,
  roadmap R1), optionally with `by X` naming where the members are stamped from
  [obligation-by]. **A `type` declaration carries the clause too**, written
  last (`type Source = A | B : ToStr<self> by auto`, `intrinsic type List<T>
  canbe Mut : Hashed<self>` — comptime round 2, R-1): a named union opts in
  like a struct, and its fulfilments attach to it the same way [fn-attached].
  Where `?Group<T>` asks the *call site* to supply the members, `: Group<T>`
  promises they exist for this type, and the promise is checked **at the
  declaration**:
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
    struct an `iter fn` generates [iter-fn]. That is what types a *bare*
    lambda over such a subject, where no declared `next` exists to resolve.
  * Two-sided unification: the *candidate's* generics bind from the known
    part of the pattern, and then the *caller's* variables bind from the
    instantiated candidate. A part that is still one of the caller's
    variables teaches nothing and must not match everything.
    * **The argument's own claims are not the candidate's business** (fixed
      2026-09-25): both directions try an exact unification first — so a `Mut`
      parameter keeps demanding a `Mut` argument — and then again with the
      *argument's* qualifiers dropped [qual-erasure]. Matching strictly meant a
      **qualified argument taught the call nothing**:
      `total(list_of(1, 2, 3))`, whose argument is `NonEmpty List<Int>`
      [col-of-nonempty], left `It` unbound, so the `?next` beside it resolved
      by rung [fn-overload-scope] and a sibling iterator's `next` won — a call
      neither backend accepts, while `total([1, 2, 3])` had always worked.
  * **Repeated until it stops learning, and once more after every argument is
    typed** (user decision 2026-09-10). That is what makes a *container*-shaped
    combinator work — `total<C, It>(c: C, ?iter: (c: C) -> Mut It holds proj(c),
    ?Yield<It, Int>) -> Int => c`, where nothing but
    the chosen `iter` says what `It` is (and the fn type's own return says
    the pass it answers holds a borrow of `c` [proj-infer]):
    `C` is only known after the arguments, so the sweeps between them cannot
    learn `It`, and one implicit determining another needs the sweep repeated.
    Declaration order is therefore not a constraint on the author.
    * The motivating case is a [iter-fn] subject, whose generated struct is
      **unnameable** — so a written type-argument list is not an available
      workaround and inferring it is the only way the shape can exist. The
      `?Iter<C, T>` spread [iter-group] is this shape with the iterator
      generic hidden.
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
* [implicit-with] **Implicits that are filled together.** A deduction clause
  entry relates two implicit parameters — `=> eq with hash` — and says they are
  one decision: a call fills every member of the relation **from one source**, or
  leaves them all alone (user decision 2026-09-26). It is written on a function,
  or on a `params` group, which is where a group states what its members mean
  together:

  ```
  fn f<T>(?func: (T) -> Bool, ?other: (T, T) -> Int) => func with other

  params Hashed<T> => eq with hash {
      fn hash(value: T) -> Long
      fn eq(a: T, b: T) -> Bool
  }
  ```

  * **Why a group would want one**: `?Hashed<T>` is the motivating case. A hash
    and an equality are meaningful only as a pair (`eq(x, y)` implies
    `hash(x) == hash(y)`), and a container buckets by the hash *first*, so a
    custom equality beside a resolved hash quietly does nothing rather than
    failing [cmp-carry]. `with` turns that into a diagnostic.
  * **The three sources** are what a call site can supply: the caller **wrote**
    the member (`eq = all_same`), the enclosing fn **forwarded** its own, or
    **resolution** found a declaration [implicit-resolve]. Mixing them across one
    relation is the error; the remedy is to **write them out**
    (`mut_set_of(hash = by_x, eq = eq)`), which states the agreement instead of
    assembling it. Where a member is *forwarded*, writing it out is impossible —
    a keyed container's identity has to be a name a type can carry — so the
    remedy there is to hold the whole group, and the diagnostic says which
    applies.
  * **Symmetric**, because the relation states that two implicits must *agree*
    and agreement has no direction. `?Hashed<T>` looks directed — writing `eq`
    alone is always wrong while writing `hash` alone is often fine — but a hash
    written against a coarse declared `eq` breaks the implication too, and the
    checker cannot tell those apart without reasoning about what a body reads.
    Where a relation genuinely *is* directed the honest statement is a
    derivation, not this word.
  * **Transitive by the check**, so the relations form **components**: "if any
    member of a component is filled, every member must be" already forces the
    closure in one pass. A **chain** is therefore one entry — `a with b with c`
    binds all three — and the diagnostic names the whole component rather than
    one missing partner.
  * **A signature may add pairs and never remove them.** A fn's own clause unions
    with the clause of every group it **spreads**, because spreading a group opts
    into its deductions along with its members. Declaring the same functions
    *individually* is how a signature takes them unbound — which is what makes
    "never remove" cost nothing.
  * A `with` naming something that is not an implicit parameter of the signature
    is an error: only a `?name` position is *filled*.

* [implicit-resolve]* [implicit-resolve]'s candidate test is **parameter-contravariant**: a
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
  * **A lowering that reads its type argument reads the position's type
    instead** (2026-09-28): a fn value has no type arguments of its own, but
    the checker records the position's type with the call's arguments
    substituted (`ImplicitArg::Resolved.want`), and for `protocol<E>` filling
    `?protocol: () -> Protocol<Ping>` that names `Ping` — so the adapter's
    body is the `Protocol` literal for it [protocol-hash]. `copy` reads the
    same field for its argument's shape [copy-implicit]. Any other intrinsic
    whose lowering needs a type argument is a codegen error as a value
    [backend-never-wrong], not a guess.
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
* [iter-fn] An **`iter fn`** is a hand-written `next` whose **iterator struct
  is generated** (user decisions 2026-09-09, redesigned 2026-09-26/27): the
  declaration is the **minter** — any name, any parameters — the `state { … }`
  block declares the struct's own fields, the body is its `next`, and the
  compiler writes the struct. Its type has no name a program can write; it is
  spelled `iter T` [iter-type]. The one named `iter` with one parameter is the
  canonical minter an `: Iter<self, T>` obligation asks for [iter-group].

  ```
  iter fn countdown(from: Int) -> Emitted Int | Finished {
      state {
          at: Int = from
      }
      if at <= 0 { return finished() }
      at = at - 1
      return emitted(at + 1)
  }

  for n in countdown(3) { … }      // 3, 2, 1
  let p = countdown(3)             // p : iter Int — hold it, drive it yourself
  ```

  * **It is a desugaring, done in the syntax crate** (`desugar::expand_iter_fns`,
    inside `parse_module`), into a hidden `struct __Iter_<fn>_<param types> :
    Yield<self, T> canbe Mut { <held parameters>, <state fields> }`, a minter
    under the `iter fn`'s own name and parameters returning `Mut __Iter_…` whose
    body is the struct literal (its lends inferred [proj-infer]), and the
    author's body as `fn next(__p: Mut __Iter_…) -> Emitted T | Finished => __p:
    Mut` with the parameters and the state fields written out as field reads (a
    `proj(xs)` in the written return is redirected to `__p` [yield-proj]; a
    dependent claim `Idx(xs)` becomes `Idx(__p.xs)` in the `next` and
    `Idx(self.xs)` in the obligation [qual-depend]). Nothing downstream knows
    the form exists, which is why `for`, the combinators, `let p = countdown(3)`,
    deductions, narrowing and both emitters need no new machinery. The struct's
    name is keyed by the fn *and* its parameter types, since an `iter fn`
    overloads like any fn.
  * **The `state` block is declarations only**, each with an annotation and an
    initializer, evaluated **once per iterator, at the mint**. The initializers
    become the body of the generated minter, which is declared `[]` — so "no
    effects in an initializer" is not a rule of its own but an ordinary effect
    error at the offending call. (The annotations are required for the same
    reason a struct field's are; a `state` field may be annotated `iter T`,
    filled from its initializer [iter-type].)
  * **The struct holds as little of each parameter as the body needs**, decided
    by a scan of the body in three tiers — all observationally identical,
    because the struct **borrows** what it holds [proj-field] and a parameter
    cannot be written while an iterator over it lives [proj-infer] (user
    decision 2026-09-11; until then the mint *copied*):
    1. the body never reads it (a plain counter): the struct holds **nothing**
       of it — unless a dependent claim in the return names it, which needs a
       place to name;
    2. it only ever reads *plain fields* of it, their types are visible (the
       struct is declared somewhere in the **program** — the expansion runs
       program-wide, local declarations winning a name) and the type is
       non-generic: one **`proj` field per field read** (a Copy scalar stays
       owned [copy-scalar-free]), initialized at the mint, keeping the field's
       own name unless a `state` field or a parameter already has it;
    3. otherwise — the value handed on, an assignment through it, a generic
       type, a declaration the program cannot see: the **whole parameter** is
       borrowed under its own name (`xs: proj List<T>`). A Copy scalar
       parameter is simply stored by value and needs no clause entry.
  * **Borrowing rather than copying** is what keeps the backends in step
    without a copy: a write during a drive is *refused* rather than differently
    visible. An `iter fn` that wants a snapshot writes one (`state { rows:
    List<Int> = copy(c.rows) }`). A *generic* parameter therefore needs no
    `copy` of a `T`.
  * **The parameters are read-only in the body**: each is a field of the struct
    typed as written, so writing through one is the standing [struct-mut]
    refusal.
  * **The generated struct is not nameable.** `__Iter_…` is the compiler's
    namespace and a *written* type reference beginning with `_` is a parse
    error, so an iterator struct a program must name is written out by hand —
    the boundary the form rests on — and `iter T` is its spelling where a
    pattern or a hidden generic is admitted [iter-type].
  * **Refused, each at the declaration**: a variadic or implicit parameter (no
    field to hold it; a stage over a *generic* source — an `iter fn` taking
    `?Iter<C, T>` or an `iter T` parameter — is recorded in ROADMAP.md), a
    `Mut` parameter (advancing never writes through it), a structural
    parameter type (array, tuple, union — the struct is named after its
    parameters), a result other than `Emitted T | Finished` (the element type
    is read out of it), a `state` field without an initializer or an empty
    `state` block, a `state` field named like a parameter, and a binding in the
    body that would **shadow** a parameter or a `state` field (it would silently
    mean something else). A `state` field of linear type is [linear-composite]'s
    ordinary refusal at the struct: **no discharger is generated** (user
    decision 2026-09-27 — it would have to be a member of the group to stay
    consistent, and it is not one), so an iterator that owns a resource is
    written by hand.
  * **Effects are ordinary effects**: each `next` is a separate call, so
    `[Console]` on an `iter fn` needs no threading of handlers across a
    suspension — a `for` passes them per turn [fn-effects].
* [iter-type] **`iter T` is the name of an anonymous iterator**: "a `Mut` type
  declaring `: Yield<self, T>`" (user decisions 2026-09-26/27). It means one
  thing everywhere, read in whichever way the position allows, and **every use
  lowers to something the program could write explicitly** — the table in
  `docs/language/Iterators.md` is the specification, one row per position.
  Lowered to a marker type (`Ty::iter_marker`, `Ty::Named { name: "iter" }`,
  displayed `iter T`) that every admitting position substitutes or fills, so
  none reaches a backend; `Mut` is implied and `Mut iter T` is refused as the
  duplicate it is [qual-no-dup].
  * **Group member** — the associated placeholder, one type across the group's
    members (`params Iter<C, T> { fn iter(c: C) -> iter T; fn next(it: iter T)
    -> … }`) [iter-group]. A member taking `iter T` is a `Mut` position, and a
    member returning it lends every kept parameter, as a fn type with an
    opaque return does [proj-infer-fn-type].
  * **Spread and parameter** — a **hidden generic**, hoisted by the desugaring
    (`desugar::hoist_iter_types`, before resolution): `?Iter<C, T>` on a fn
    appends a fresh `__ItN` to its generics and to the spread's arguments
    (`?Iter<C, T, __It0>`), which the checker substitutes for the placeholder
    in every member; a parameter `it: iter T` becomes `it: Mut __ItN` plus a
    `?Yield<__ItN, T>` spread — fresh **per occurrence**, so `chain(a: iter T,
    b: iter T)` takes two different structs (two parameters that must be the
    same struct write `<It>`). Hidden generics render like any other in both
    targets; an explicit type-argument list at a call names the visible ones.
  * **`let` annotation, `state` field, return of a fn with a body** — a
    **pattern** filled from the value: the annotation asserts "an iterator of
    `T`" (an iterator struct declaring `: Yield<self, T>`, a walking one's `proj
    T` counting as `T`; a hidden generic with a `next` over it in scope) and the
    binding keeps the concrete type; nothing is widened. A `-> iter T` fn
    returns whichever concrete struct its body mints — callers see that type
    through `fn_return_ty`, a **pre-pass** in `check_once` checking those
    bodies first into a scratch `Checked` and repeating until the
    `iter_returns` table stops growing (a delegating minter may call another
    pattern fn) — and every path must mint the *same* struct: two anonymous
    ones are two types, and a second differing `return` is an error naming both
    with the remedy "fold the case into one `iter fn`, or name the struct". A
    cycle the pre-pass cannot resolve is an error at the signature. The
    resolved type is recorded per written span (`iter_types`) for the emitters
    and hover.
  * **Type of an `iter fn` call** — the concrete anonymous struct, which prints
    as `iter T`; two from different minters never unify.
  * **Refused, with the replacement named**: the return of a bodiless fn (no
    body to fill it from), an effect member's parameter or return (implemented
    per handler, one interface type needed), a fn *type* (the value's struct is
    its own to choose), a struct or handler field (storing "some iterator
    struct" is boxing). The remedy is the struct's name, or a generic on the
    enclosing declaration.
  * **Linear**: no sugar for a linear *iterator* — the struct is written out
    with its discharger, and generic code that may receive one takes the
    discharger as a callback [linear-generics]. Linear *elements* are fine:
    `iter T` with `T canbe linear` needs no discharger.
  * **Reachability** [mod-used-only]: the placeholder names no declaration, and
    `iter` is every container's minter, so `reach.rs` skips the name — before
    that every mention of the placeholder pulled every container module in.
* [iter-group] **`params Iter<C, T>` is what a source is** (user decision
  2026-09-26): `fn iter(collection: C) -> iter T` and `fn next(iterator: iter
  T) -> Emitted T | Finished => iterator: Mut`, tied `iter with next`
  [implicit-with] — a caller may override the minter, never the step alone,
  since the step belongs to whatever the minter answers. `Yield` and `Iter`
  are one protocol stated from two sides: the second member of `Iter` is
  `Yield`'s `next` over the placeholder.
  * **The obligation** `struct Bag : Iter<self, T>` is satisfied by an `iter fn
    iter` over the type (the compiler writes the struct and both members) or by
    a fn `iter` of that arity over `self` whose result is a type declaring
    `: Yield<self, T>` (`iter_minter_satisfies`, matching the element up to a
    renaming and modulo `proj`) — the "declared" reading, so a `P` without the
    clause never satisfies it. The `next` member is satisfied by construction.
    A struct declaring both `: Iter<self, T>` and `: Yield<self, T>` is refused:
    `for x in s` would have two answers (drive `s`, or mint from it).
  * **The spread** `?Iter<C, T>` brings in `iter: (C) -> Mut __It` and `next:
    (Mut __It) -> Emitted T | Finished` over the hidden generic [iter-type] —
    exactly the 2026-09-10 container-shaped combinator (`total<C, It>(c: C,
    ?iter: (c: C) -> Mut It, ?Yield<It, Int>)`) with `It` hidden, so both
    backends already lowered it. Inside the body, `for x in c` over the generic
    source mints with the `iter` implicit and drives with the `next` beside it
    (`PassDriver.mint = Some(PassMember::Implicit("iter"))`,
    `generic_pass_elem_ty`); `iter(c)` is a `Mut __It` a `?Yield` combinator
    accepts with the paired `next` forwarded [implicit-forward]. The pair is
    filled at the call from whatever `C` is: an `iter fn`'s minter, a named
    struct's, or std's own for an intrinsic container.
  * **Lowering the lend** [rs-proj-lends]: the `iter` position's parameter is
    named after the *group member's* (`collection`), not the spreading fn's, so
    the Rust backend ties a lent position with a foreign name to the one
    enclosing parameter of its type.
* [iter-step-call] **`for` over a step call** (user decision 2026-09-26): `for
  i in next(p)`, `for i in skip(z)` — a subject that is a *call* whose result
  is `Emitted T | Finished` is re-invoked each turn until `Finished`, which is
  what lets a **non-canonical** step (a second one over the same struct, one
  under any other name) be driven at all. The general form of `for`: the
  canonical drive is `for i in next(p)` with `next` resolved by the
  declaration.
  * The arguments are **evaluated on every turn** as written, so each must be a
    **place or a literal** (`is_step_call_arg`: a variable, a field path, a
    literal) — bind anything else with `let` first — and the callee must
    **keep** every one of them: a step that consumes its argument cannot be
    called twice on it (the general form of "`next` takes `Mut It`"). Both are
    errors at the loop.
  * A *value* of that shape that is not a call (`let r = next(p); for i in r`)
    is an error: a union is not iterable, and only a call can be re-invoked.
  * Recorded as `PassDriver { step_call: true, .. }` keyed by the subject's
    span; both emitters render the call as written inside the loop header
    (Rust `while let Union2::U1(i) = skip(&mut z) {`; Kotlin re-invokes into
    the step local), so argument modes follow the ordinary call rules
    [rs-borrows].

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
    as an effect list on the handler and captured as handles at construction
    (user decisions 2026-09-04, 2026-09-14 and 2026-09-28; mechanism in
    [rs-handle] / [kt-handle]). A *member* declaring its own
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
  implementations differ in what they need (user decision 2026-09-04). What
  *every* implementation needs is an effect prerequisite instead
  [effect-prereq]. The list is written in the handler's file and **resolves
  in that file's scope** wherever the handler is bound (fixed 2026-09-29: it
  used to be lowered against the binding file's imports).
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
    acyclicity guarantee, which is what the lock discipline relies on.
  * **Emission is one form** ([effect-handle], user decision 2026-09-28): a
    dependent handler captures its dependencies as **handles at
    construction** — fields and trailing constructor arguments, one per dep
    in declaration order, whatever the dependency's kind (plain, actor
    effect, generic instance) and however the handler is bound
    (`Checked::use_deps` / `spawn_deps` record the resolved instances per
    construction; the emitters read only those). The handle comes from a
    binding in the same function or from an effect the function received
    through its own signature — a fn's effect parameter *is* a handle
    ([spawn-inherit]). A **generic** dependent handler works (its dep is a
    handle field, so nothing generated has to name the handler's generics).
    Rust: [rs-handle]; Kotlin: [kt-handle] (an object reference is the
    handle, `__Mon_E` where a stateful one is shared). The two forms this
    replaced — owned handles for the shareable default and a per-scope
    *fusion* threading the dependency per call for every other dependent
    handler, chosen by `handler_handle_deps` — are recorded in COMPLETED.md
    ("One shape for effects").
* [effect-prereq] **An effect may name prerequisites: `effect Fs [Streams] { … }`**
  (user decision 2026-09-29, R2 of the stream layering; built the same day).
  The spelling is a handler's dependency list, which already means "needs this
  from the enclosing scope". A **prerequisite, not inheritance** — the user
  first proposed `effect Fs : Streams` meaning inheritance, and inheritance
  would give every handler of `Fs` its own `Streams`, which is exactly the
  split the stream layering exists to remove.
  * **Effect lists**: an entry naming `Fs` — in a fn's list or a fn *type*'s —
    implies `Streams`, transitively, with a generic effect's arguments
    substituted (`Log<T> [Sink<T>]` at `Log<Str>` implies `Sink<Str>`).
  * **Handlers**: every non-platform, non-intrinsic handler of `Fs` gains
    `Streams` as a dependency, unless it wears `Streams` as a face itself
    (`of Fs, Streams`). A `use` without a `Streams` bound reports the
    prerequisite by name (`handler \`MemFs\` implements \`Fs\`, which needs
    \`Streams\` in scope`), recognised by the implied entry's synthetic span.
  * **An expansion before resolution** (`prereq::expand_prerequisites`, the
    last step of `expand`): the implied entries are written into the AST with
    an empty span at offset 0, so the checker and both emitters see an
    ordinary program and agree by construction. A module that must name an
    implied effect it cannot see gets a synthetic `import` of it (same span).
    Name resolution inside the pass is a reduced copy of the resolver's
    ladder for effect names (own module, the tested module for an annex, a
    named import by last segment or alias, a whole-module import, `core.*`).
  * **Refused at the declaration**: `use`/`spawn`/`any E` entries; an unknown
    name; an `actor effect` with prerequisites, or one as a prerequisite; an
    exported effect with a private prerequisite; a cycle (named as a chain).
    At a site: a module whose own `Streams` would shadow the implied one.
  * Found building it: a handler's dependency list was lowered in the
    *using* file's scope, so any dependent handler bound from a module that
    did not import its dependencies failed with "unknown effect" — fixed at
    the root [effect-handler-deps]; and the Rust emitter's argument hoisting
    took a path segment (`crate::files::Files`) for a use of the effect
    variable `files`, hoisting a closure into a `let` where rustc cannot infer
    its reference parameters as higher-ranked — `mentions_ident` now skips
    path segments.
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
  * Refused, each by name: an item the handler does not depend on (supplied
    for nothing); a **multi-face** handler as an item (the clause builds one
    instance per item, so the second face has nowhere to go — bind it with
    its own `use` and name the addr); an item with **dependencies of its
    own** (a clause item has no scope to resolve them from — register it
    before this one instead); and a `with` on a `use` of an **existing
    handle** (its dependencies were settled where it was built). The refusal
    of a `with` on a handler with an actor-effect or generic-instance
    dependency went with the fused emission (2026-09-28).
  * `with` is a keyword already — the qualifier-compatibility clause spells
    it (`qualifier Q of T with A`) — and the two positions cannot be
    confused: one follows a qualifier's `of` type, the other a `use`/`spawn`
    handler expression. Same-line, like every trailing clause.
* [spawn-inherit] **A spawned handler inherits the spawning scope's
  dependencies** (user decision 2026-09-20; the arc the shareable-by-default
  round was built for). A declared dependency the `with` clause does not
  cover is resolved from the scope's availabilities and captured as an owned
  handle that travels with the child — which is sound *modularly*, with no
  whole-program analysis and no runtime check, exactly because every binding
  is a handle [effect-handle]. The old rule ("dependencies come from the
  clause, never from the spawning scope") is gone; the clause is for
  overriding [with-clause].
  * Transitional (ROADMAP §2b): a binding the emitters still bind *inline*
    (an actor-face handler's `use H()`, a handler with an actor-effect or
    generic-instance dep) has no handle yet, and the refusal names the
    remedy — for an actor effect, "spawn it and bind the addr"; otherwise
    supply the child its own with `with`.
  * **Ambiguity is an error, not a guess**: two instances in scope that
    could both satisfy the dependency (a generic effect with an unpinned
    instantiation) is reported, naming `with` as how the program says which.
    An exactly matching instance always wins, so this only fires where
    unification is doing the work.
  * **Nothing in scope** stays an error, now naming both remedies (bind one
    before the spawn — it is then inherited — or supply it in the clause).
  * **The v1 lexical cut is lifted.** A capture used to require the
    dependency to come from a `use` *in the same function*; an effect
    arriving through the enclosing **signature** works too, since every
    effect parameter is a handle [effect-handle] — a fn value's effects
    included (until
    2026-09-28 those two were refused, and the lift went through a hidden
    handle-bundle parameter on Rust; see COMPLETED.md).
  * **The deadlock graph is unchanged**: it prices a handler's dependencies
    from its *declaration*, so an inherited dependency carries exactly the
    edges a written one did [actor-deadlock-cycle].
* [effect-handle] **Every binding of an effect is a handle** (user decision
  2026-09-28, *one shape*): a `use` makes one value — the handle of the
  instance — and everything downstream is that value or a copy of it: a
  fn's `[E]` parameter, a dependent handler's captured dependency
  ([effect-handler-deps]), a spawn's or task's inherited effect
  ([spawn-inherit], [task-effects]), a fn value's effect. A `use` of an addr
  or of a spawn expression is already a handle.
  * **Stateless or stateful is the one distinction the handle keys on**,
    on both backends (user decision 2026-09-28, the afternoon's refinement):
    a *stateful* handler's members run under a lock through its handle — the
    monitor, [monitor-handler] — so two spawns capturing one stateful
    binding share its state (accepted as a consequence of the shape); a
    *stateless* handler's members run on the shared instance with nothing
    held, so a send through an actor's bound addr, a mixed handler's façade
    waiting on its servant, and every stateless handler cost no lock. Read
    off the declaration by one predicate (`salvo_core::handler_is_stateful`):
    a `state` field; a member that mints a `replyto` (the parked table is
    state); a **fn-typed constructor parameter** (a stored callback may
    mutate its captures); a **platform handler without `threadsafe`**
    ([threadsafe-platform] is the declared statefulness of a host the
    compiler cannot see). An `intrinsic` handler is trusted stateless, and
    an actor handler's generated address and parked table do not count (the
    actor body owns the instance). The deadlock graph's `H's lock` nodes
    follow the same predicate ([actor-deadlock-cycle]).
  * **On Rust the handle carries the effect's name** ([rs-handle]): `fn
    work(console: &Console)`, the two traits a handler implements being the
    mangled `__Stateless_E`/`__Stateful_E`; on Kotlin the interface is the
    type and a stateful binding is wrapped in `__Mon_E` ([kt-handle]).
  * **Replaces** the 2026-09-20 shareable-by-default round's two spellings,
    both deleted from the language: `use local H(args)` (the scope-local,
    lock-free binding) and `local E` in an effect list (the requirement that
    accepted one, with its call-site rule and its viral spread down call
    chains). The record is in COMPLETED.md's log; a lock-free scope-local
    binding returns as an optimisation, not as syntax (ROADMAP, "Recorded,
    not scheduled").
  * A fn *type*'s effects are its parameters like any fn's, so a lambda's
    availabilities are handles too and a fn called from inside a lambda is
    written like any other.
  * **Lending members need no refusal**: an effect member that lends a
    mutable view ([rs-loc]'s wholesale-lending shape) lends from a
    *parameter* — an effect declaration has no `self` to name — so the borrow
    outlives the handle's lock legitimately and the handle forwards both
    faces. (ROADMAP §2b planned a checker error for lending from handler
    state; no program can write one.)
* [effect-any] **`any E` weakens identity** (user decision 2026-09-26, the
  network round). Bare `[E]` keeps
  the strong meaning every program so far assumed: *one* instance, sends in
  order, state shared between them. `[any E]` says "each send may go to a
  different instance; I assume no order between my sends and no state
  across them" — and accepts every binding, since a single instance is a
  group of one. Contextual: `any` followed by an effect name.
  * **The binding side**: a handler declares the weaker guarantee in its
    `of` clause, per face — `handler RoundRobin(members) of any Resizer` —
    and a `use` of it binds `any Resizer`, which satisfies **only**
    `[any Resizer]`. A router that forwards to many members and omits `any`
    is telling a lie the checker cannot see: the one gap, accepted.
  * **The call-site rule**: a bare `[E]` requirement
    against an `any` binding is an error naming both remedies (declare
    `[any E]` if any member may take each send, or bind one instance); a
    strong binding satisfies both forms. **Viral downward**: a body holding
    `[any E]` may call `E`'s members and callees declaring `[any E]`, never
    a callee assuming one instance; a body holding `[E]` may call either.
  * A router is the one handler of an **actor effect** whose `use` binds
    shareable (bare when stateless, a monitor when stateful): it forwards
    to members and holds no protocol state of its own, and the emitters
    give an actor effect the lock adapter exactly when a router of it
    exists. It is also a servant node of its own in the deadlock graph, so
    its forwards do not read as `E → E`.
  * **A fn type carries no strength** (it cannot say `any`): an inherited
    requirement [fn-effects] is the strong one unless
    the function taking the value writes `[any E]` itself, which weakens it
    — the written entry wins the dedupe. Recorded gaps: an `Addr<E>`
    answered by *spawning* a router binds as an ordinary `E` under `use
    addr` (the addr type has no room for the claim), and a lambda's body is
    checked under strong availabilities.
* [effect-no-dup] Two effects of the same type in one list are an error
  unless their generic arguments differ (`[Random<Int>, Random<Double>]`
  is fine, `[Console, Console]` is not).
* [effect-use] `use Handler(...)` registers a handler instance for the
  rest of the current scope; `use Handler` is sugar for `use Handler()`.
  * The checker infers the handler's generics from the constructor
    arguments and registers the *concrete* effect instance
    (`use CyclicRandom(list_of(1,2,3))` registers `Random<Int>`), recorded
    in `use_effects`.
  * **Each argument must fit its constructor parameter**, by the same relation
    a call's argument is judged by (`arg_fits_param`, in an owned position —
    a constructor argument is stored [deduce-consume]). A handler has no
    overload set, so nothing else was reporting this: a parameter demanding a
    claim (`numbers: NE List<Double>`) used to accept a value without it, and
    the disagreement surfaced later or not at all.
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
  * Emission: the previous registration's handle is captured *before* the
    interceptor is constructed and bound — `let mut logger2 =
    __Handle_Logger::new(Stamped::new(logger.clone()))` — which is "binds
    strictly outward" in emission ([rs-handle], [kt-handle]).
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
    `Fs` member per stream token *and* the `Lines` iterator's discharger
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

### Comparison, equality and hashing (std, `core.compare`)

* [cmp-groups] The three capabilities are **params groups**, not traits (user
  decisions 2026-09-21, the ordering round): `core.compare` declares
  `Ordered<T> { fn cmp(a: T, b: T) -> Int }`,
  `Eq<T> { fn eq(a: T, b: T) -> Bool }` and
  `Hashed<T> { fn hash(value: T) -> Long }`. Having a capability is "a fn of
  that shape is in scope" and nothing more — so a type joins in by *declaring
  a function*, a generic fn asks for one with `?Ordered<T>`
  [implicit-group], the call site resolves it [implicit-resolve], and neither
  backend learns that any of this exists [group-not-a-value].
  * **The contracts are trusted, not checked**: `cmp` is a total order whose
    zero agrees with `eq`; `eq` is an equivalence; `hash` agrees with `eq`
    ([cmp-hash-values]). What *is* checked is the ordinary thing — a call
    resolves to exactly one visible overload, or says so.
  * **The canonical implementations for the intrinsic types are `intrinsic`
    overloads** [intrinsic-fn], one per type: `cmp` for `Int`, `Long`, `Byte`,
    `Char`, `Bool` and `Str`; `eq` for those plus `Double` and `Float`; `hash`
    for the ones `cmp` covers. So `Double`/`Float` compare for equality and
    have no ordering and no hash — `NaN` ties with nothing, so no total order
    exists and a float key would hash unpredictably, which is the same
    reasoning [col-sorted] and [col-hashed-ordered] already apply.
  * **`Str` compares by code point** on both backends, the contract
    [col-sorted] already owed: Rust's `str: Ord` is byte-wise UTF-8 (which
    *is* code-point order), while the JVM's `String.compareTo` is code-unit
    order and is corrected in the lowering [kt-ordered].
  * A **type of your own** joins a capability by declaring the fn
    (`fn cmp(a: Person, b: Person) -> Int`), reached by bare call or by
    dot-notation [fn-dot] like any overload. The obligation form
    `struct Point : Ordered<self>` checks at the declaration that one exists
    [group-obligation] [group-self] — it designates nothing and adds no scope.
  * A **generic** fn reaches the capability only by asking: nothing about an
    opaque `T` is knowable [call-resolve], so `?Ordered<T>` (or an individually
    declared `?cmp`) is what makes `cmp(a, b)` legal there, and it forwards by
    name and type through further calls [implicit-forward]. A group's member
    *keeps* its parameters (no deduction clause keeps everything
    [fn-contract]), which is what lets a body compare two values and still own
    them.
  * **Not yet the operators.** `<`/`==` still resolve as [op-order] and
    [col-equality] describe; routing them through these groups (and with it
    opt-in equality, the `default` generator, `@`-scoped canonicals and
    ordering-carrying types) is the rest of the round — see ROADMAP.md
    "Ordering, equality and hashing".
* [fn-attached] **A function is declared *on* a type by being declared with
  it** (user decision 2026-09-26, replacing the `fn cmp@Person(…)` spelling).
  Two routes, and they are the same idea — the two ways to write something down
  on a struct:
  1. **inside the struct's body**: `struct Person { name: Str, fn cmp(a: Person,
     b: Person) -> Int { … } }`. The fn is hoisted to module level and is
     ordinary in every other respect (it overloads, it is called bare or with
     dot notation, it takes part in ranking);
  2. **the file's fulfilment of one of the type's obligations**
     [group-obligation]: `struct Person : Ordered<self>` plus a `cmp(Person,
     Person)` in the same file. Nothing moves; the obligation is what declares
     the capability, and the function that answers it is on the type.

  The rule this produces is the one worth remembering: **everything declared on
  a struct is imported with it**. The shape is the one actors and qualifiers
  already use — a declaration's members live in its body — and it is why the
  `@` on a declaration is gone: a selector said the same thing in a second way.
  * **It travels with the type.** Importing `Person` imports every fn declared
    on it, so an attached fn is in scope wherever the type is usable. That is
    what closes [implicit-resolve]'s per-call-site visibility hole: a module
    importing `Person` but not its file's `cmp` could otherwise resolve `?cmp`
    to some *other* visible `cmp(Person, Person)`, or to nothing. The fn keeps
    its own name — an `as` alias renames the type, and a partial rename of a
    capability would mean nothing.
  * **Attachment is same-file by construction**, which is what makes "imported
    with the type" applicable without searching the program: a body is only the
    declaring file's to write in, and an obligation is fulfilled by its own
    file. No third module can mint a capability for someone else's type.
  * **`export`**: an inner fn takes the struct's own visibility, and writing
    `export` on one is a parse error (it is contained in the struct, and would
    be claiming to say something the language does not let it say). A
    **detached** fulfilment is its own declaration, so an exported type's
    fulfilment must `export` too — otherwise the capability would be
    unreachable wherever the type is usable [mod-export]. A *private* type
    demands nothing: there is no importer to hide it from.
  * **It is the default selection for an implicit parameter** of the same name
    and shape [implicit-resolve].
  * **The selector still reads it**: `cmp@Person` at a call names the `cmp`
    declared on `Person`, and so does `cmp = cmp@Person` in value position
    [implicit-override] [fn-overload-at]. Only the *declaration* spelling
    changed. Naming a type with no such fn is an error saying where one would
    live.
  * **Same name, same parameter types is a duplicate** whether attached or not
    [fn-overload-duplicate]: nothing at a bare call site could tell the two
    apart, and the remedy is to keep one. (That is also what catches a
    hand-written implementation colliding with a generated one.)
  * The selector is **erased**: the checker records which declaration a
    selected name means, and both backends emit an ordinary call of it.
  * For the intrinsic types the canonicals stay plain std overloads: `Int` is
    declared in `core.basic` and `cmp(Int, Int)` in `core.compare`, so neither
    route is available — and `core.*` is implicitly visible everywhere, which
    is the travelling that attachment exists to provide.
* [obligation-by] **`by X` on an obligation clause stamps every member of the
  group from `X`'s `comptime fn`s** (user decisions 2026-09-28, the comptime
  rounds; replaces `auto`, which is deleted from the grammar):

  ```
  struct Point : Ordered<self> by auto, Hashed<self> by auto { x: Int, y: Int }
  export type Source = Manual | Imported : ToStr<self> by auto
  ```

  `X` is a **module**, named by any unambiguous suffix of its path
  [mod-suffix] so `by auto` reaches `core.auto` as `size@list` reaches
  `core.list`, or a `comptime fn` named directly [fn-by]. **A name that is
  both** is refused as ambiguous rather than defaulted, and each reading has a
  spelling (user decisions 2026-09-29): `by @auto` is the module — a module
  stands on the right of an `@`, as it does in `size@list` — and `by
  auto@mymod` the comptime fn `auto` declared in `mymod`, the call selector
  [fn-overload-at] reused. For each member of the group the comptime fn of that name
  **and of the type's kind** (`<T is Struct>` for a struct, `<T is Union>` for a
  named union) is instantiated at the type, and the result is an ordinary fn
  **declared on the type** [fn-attached]: it travels with it, `cmp@Point`
  names it, `?cmp` resolves to it by default, and it carries the type's own
  `export`.
  * **Nothing applies on its own.** A struct with no `cmp` cannot be compared,
    and a struct with no `to_str` does not interpolate; the clause is the one
    line that opts in, and the diagnostic at a bare `${p}` names it.
  * A `type` declaration carries the clause too, **after** the alias — which is
    how a **named union** opts in (comptime round 2, R-1). A free union
    (`Str | Int`, every `T?`) has no declaration and no functions of its own;
    a comptime fn over a struct reaches its arms through the field
    [comptime-inline].
  * The clause's argument is `self` [group-self]; two groups asking for the
    same member (`Eq<self> by auto, Hashed<self> by auto`) ask for one
    declaration.
  * **Where the stamping fails** — a field with no `cmp` in scope, a `refuse!`
    — the error lands at the *field* the copy was for, prefixed with what was
    being stamped [comptime-instantiate]. A hand-written member of the same
    shape beside a stamped one is the ordinary duplicate
    [fn-overload-duplicate], with the clause named as the half to delete.
  * The clause without `by` is only a promise, checked at the declaration
    [group-obligation]: what satisfies it may be a stamped fn (`fn cmp(…) by
    auto` in the body) or a hand-written one, which is how a type mixes the
    two.
  * [comptime-generic] **A generic target is stamped generically** (decided
    with §2c, built 2026-10-05, ROADMAP §0j 6i): `struct Wrapper<T> :
    Ordered<self> by auto` stamps `fn cmp<T>(a: Wrapper<T>, b: Wrapper<T>,
    ?Ordered<T>)` — the target's type parameters and their `canbe` opt-ins,
    and one group per parameter a field (or arm) mentions: `?Ordered<T>` for
    `cmp`, `?Eq<T>` for `eq`, `?Hashed<T>` for `hash`, `?ToStr<T>` for
    `to_str`. A template of another name is still refused at a generic
    target. The template's own type parameter is renamed out of the way first
    (`T` → `__Self`) when it shares a name with the target's. A concrete field
    beside a generic one reaches its own member [implicit-resolve-body].
    * Each stamped member gets a name span of its own (redirected to the `by`
      site), since one site stamps several and a fn's implicits are keyed by
      its name.
    * The obligation's match ignores the trailing implicits
      [group-obligation].
    * **Calls fill them**: a written call as always, an operator
      [op-order] [op-equality] at the comparison's span, an interpolation
      [interp-to-str] at the zero-width span after the value, and inside a
      stamped `to_str` an interpolated `T` renders through the implicit
      (`Checked::interp_implicit`).
    * **A fn value carries them by filling them in turn**
      [implicit-recursive]: a candidate with implicits of its own (a spread
      group included) fits a position when its explicit parameters do and its
      implicits can be filled, so the hash of a `Wrapper<Str>` is a stamped
      `hash` closed over the `Str` one.
    * Not yet: a generic union whose arm is a generic struct
      (`type Either<A> = Box<A> | Int`), whose stamped body calls `to_str`
      with a `Box<A>` that both the union's and `Box`'s overloads fit, which
      overload ranking does not order.
  * **Lowering**: a stamped fn is emitted as an ordinary fn whose body is the
    unrolled Salvo — a `cmp` per field, in declaration order. No structural
    member is lowered to a host derive, and no backend derives `Hash`/`Ord`
    or generates a `Comparable` for a struct: a `List<Point>` or `(Int,
    Point)` **key** reaches the element's Salvo fn [col-hashed-ordered].
* [fn-by] **`by X` on a fn declaration stamps that one member**, keeping the
  full written signature [decl-explicit]:

  ```
  struct Person : Hashed<self> {
      name: Str,
      age: Int

      fn hash(value: Person) -> Long by auto
      fn eq(a: Person, b: Person) -> Bool { return a.age == b.age }
  }
  ```

  The type stamped at is the **first parameter's** (a struct or a declared
  union); the written parameter types and return type must equal the
  instantiation's, positionally, and a mismatch names which. A body beside
  `by` is a parse error. The declaration attaches as any inner fn or
  fulfilment does [fn-attached]. `by` is contextual: it closes a type
  sequence, since a type or qualifier is uppercase by rule [name-casing].
* [comptime-bound] **`comptime fn` is a compile-time function** (user decision
  2026-09-29: one modifier, `comptime`, for the whole facility — `comptime fn`,
  `comptime struct`, `comptime type` — replacing the standalone `compfn` of the
  day before): the one scope the comptime syntax is legal in, and **not
  callable** — a generic one exists only to be instantiated by `by`, and the
  expansion removes it before resolution (COMPLETED.md's comptime entry,
  decision 2). Its type parameter carries a **kind bound spelled as the
  narrowing it is**, `comptime fn cmp<T is Struct>(a: T, b: T) -> Int` or `<T
  is Union>` — the same `is` a `[when field.type]` arm tests, against an arm of
  `core.comptime`'s `Type` — at most one, and a module's same-named comptime
  fns of different kinds are the overloads a `by` picks between by the
  target's kind. A `comptime fn` **with no bound** is concrete — its own single
  instantiation — and is declared where a fn is (in a struct body or at top
  level), which is the one-field-by-hand case (comptime round 2). Like an
  `intrinsic fn` it writes its effect list, deductions and return type; an
  instantiation reached by implicit resolution must be effect-free
  [implicit-fn-only]. `core.auto` holds `cmp`, `eq`, `hash` and `to_str` for
  structs and unions; a user module may hold its own, reached by `by mymodule`.
* [comptime-fields] **What a comptime fn may know about a type is a declared
  model, `core.comptime`** (user decision 2026-09-29, round 8): compile-time
  structs, readable and hoverable, never values —

  ```
  comptime struct Struct  { name: Str, fields: List<Field>, mutable: Bool }
  comptime struct Union   { name: Str, arms: List<Arm> }
  comptime struct Tuple   { elems: List<Type> }
  comptime struct FnType  {}
  comptime struct Basic   { name: Str }      // declared `intrinsic type`
  comptime struct Generic { name: Str }      // a type parameter of the struct being stamped
  comptime type Opaque = Basic | Generic
  comptime type Type   = Struct | Union | Tuple | FnType | Opaque
  comptime struct Field { name: Str, type: Type, index: Int, first: Bool, last: Bool }
  comptime struct Arm   { name: Str, type: Type, index: Int, first: Bool, last: Bool }
  ```

  Inside a body the bound `T` is a `Struct` or `Union` by its bound, `T.fields`
  / `T.arms` the sequence a `[for …]` walks (arms in declaration order, `None`
  included [union-arm-identity]), `T.name` and `T.mutable` its projections; a
  binder is a `Field` or `Arm` with its five projections; `field.type` is
  usable **as a type** wherever one is written inside the body, and
  `field.type.arms` reaches a union field's arms. A concrete comptime fn names
  its type directly (`Reading.fields`). Nothing else is exposed — no attached
  functions, no qualifiers on the value, no defaults yet (`field.default` is
  recorded for the codec slice).
  * **Compile-time only.** `comptime struct`/`comptime type` are never emitted,
    have no wire form, cannot be constructed, and are refused in an ordinary
    type position where they are written; only another `comptime struct`'s
    fields may name them. The compiler's own kind classification is what fills
    them; the declarations mirror it, and `type` is accepted as a field name so
    `Field.type` can be declared.
  * **Hover** (2026-09-29): a comptime fn body is never checked as itself, so
    the expansion records each comptime name's type at its written span — `T
    is Struct`, `field: Field`, `field.type: Type`, `T.fields: List<Field>` —
    and the language server answers from that record before its usual tables.
* [comptime-inline] **The comptime constructs are written in brackets** (user
  decision 2026-09-29, replacing the `inline` word): a bracket appears exactly
  where the same text without it would be runtime Salvo with another meaning
  — `[for …]` against a runtime loop, `v.[field]` against a field literally
  called `field` — and where no runtime reading competes (`field.name`,
  `T.fields`, `is Struct`, `else`) nothing is bracketed.
  * `[for field in T.fields] { … }` — one copy of the body per field, each
    checked with `field.type` concrete; locals a copy declares are renamed so
    two copies in one block do not redeclare [var-no-shadow].
  * `[if <cond>] { … } else { … }` — kept or dropped per instantiation; the
    dropped branch is never checked. Conditions: `X is Struct` (an arm of
    `Type`), `X is <Type>` (equality up to alias expansion, qualifiers erased),
    `T.mutable`, `field.name == "literal"` (a name no field has is an error at
    the declaration), `field.first`/`field.last`, `x.index == y.index` and its
    orderings, and `!` of any.
  * `[when field.type] { is Struct { … } is Union { … } is Tuple { … } is FnType
    { … } is Opaque { … } }` — an ordinary `when` over `core.comptime`'s `Type`,
    **exhaustive** over its arms unless an `else` closes it [when-exhaustive],
    so that a kind added to the language is an error in every comptime fn that
    did not consider it. `is Opaque` matches either of its arms, `Basic` and
    `Generic`, the way `is Person` matches both arms of `Surname Person |
    Person`.
  * `[when value] { [arm] { … } }` — a dispatch over a **union value** (a field
    read `v.[field]`, or a parameter of the bound or written type): the one
    written arm is stamped once per declared arm with `value` narrowed to
    `arm.type`, producing an ordinary exhaustive `when`.
  * Every form is legal only inside a `comptime fn`; elsewhere it is a parse
    error naming the scope. An interpolation inside a comptime fn body may read
    the comptime forms like the body around it.
* [comptime-access] **`v.[field]`** reads the field an enclosing `[for …]`
  is at (the binder in brackets), rewritten by the expansion to `v.name`;
  **`[field]: expr`** is the same entry in a struct literal, and `inline for
  field in T.fields { [field]: … }` inside a literal produces one entry per
  field — how a comptime fn builds a `T`, exhaustively by construction (parsed and
  expanded; its first customer is the codec slice).
* [comptime-refuse] **`refuse!("…")`** is an error at the instantiation site
  (the `by`), in the caller's terms: the literal text with `${T.name}`-style
  names substituted, prefixed "`P` refused:". Spelled with the bang and parens
  of `assert!`/`unreachable!` [assert-fn] — a place that fails (user decision
  2026-09-29). Legal only inside a comptime fn. `core.auto`'s `hash` refuses a
  `canbe Mut` struct this way (`[if T.mutable]`), which is the rule `canbe
  hashed` used to carry and the compiler used to check.
* [comptime-instantiate] **Instantiation is concrete and happens at a `by`
  site only** — an obligation clause or a fn declaration (a third site, an
  implicit override `eq by auto` at a call, is decided and deferred; ROADMAP
  §2c). One specialization per (comptime fn, type), pushed into the target's
  module beside the type on both backends. The expansion runs over the whole
  source set before resolution (`salvo_core::comptime`), so a `by auto` in any
  file reads `core.auto` and the target's declaration.
  * **Spans**: every node of a stamped body is given a synthetic span **past
    the end of the file**, one fresh region per unrolled copy, so the checker's
    span-keyed side tables never see two copies as one node. The `Stamp` on
    the fn records the regions; the checker redirects a diagnostic inside one
    to the field or arm the copy was for and prefixes it — "in `cmp` from
    `auto` for `Point.y: Double`: no matching overload for `cmp(Double,
    Double)`" at the field declaration.
  * A stamped fn is an ordinary `FnDecl` for everything downstream:
    resolution, the duplicate check, mangling, export, both emitters.
* [cmp-carry] A structure that **holds** an ordering (or a hash, or an
  equality) names it as a **fn-valued type argument**, fixed at construction
  (user decision 2026-09-21, the ordering round's decision 1). The identity lands *in
  the type*: `Heap(min_by_age) Mut List<Person>` and
  `Heap(max_by_age) Mut List<Person>` are different types that refuse to mix,
  and [implicit-resolve]'s per-call-site locality stops being a hazard because
  what one call site resolved is published by the type it produced.
  * The declaration writes a **fn slot** in its generics list —
    `qualifier Heap<T>(?cmp: (T, T) -> Int) of List<T>`,
    `intrinsic type SortedSet<T>(?cmp: (T, T) -> Int)` — spelled like the implicit
    parameter it is resolved as [implicit-param]. **There is no default to
    write**: the slot's *name* is what an unwritten one resolves by, which is how
    an implicit parameter already works [implicit-resolve] (user decision
    2026-09-22 — an earlier `= cmp` form restated the mechanism and is gone). A **`params` group spreads into a slot list** exactly as it
    does into a parameter list (user decision 2026-09-22):
    `qualifier Heap<T>(?Ordered<T>) of List<T>` declares one slot per member, at
    the position the spread is written, and two entries asking for the same
    position at the same type merge [implicit-group].
  * A use site fills the slot with an **identity**: a bare name
    (`Heap(min_by_age)`), an `@`-scoped canonical (`Heap(cmp@Person)`
    [fn-attached]) or the signature's **binder** (`Heap(?cmp)`, below).
  * **Static identity is the load-bearing restriction** (decision 12): a fn
    bound into a type must be **named, top-level and capture-free**. Module
    fns, `@`-scoped canonicals and intrinsics qualify (the last through
    [implicit-intrinsic]'s adapters); a lambda or a fn-valued local is refused,
    with the error naming why — "a lambda or local has no identity a type can
    carry; declare it as a `fn`". Everything a type can *print* it can carry,
    so widening this later is purely additive.
  * Dropping such a qualifier is **fail-safe**: the operations demand it and a
    plain value never regains it by subtyping [qual-constructive], so a lost
    `Heap(f)` costs access, never correctness.
  * Identities live in their own domain, beside types: they compare by name
    (`min_by_age` is not `max_by_age`), they print by name wherever a type
    prints, they substitute like type arguments, and no value ever has one as
    its type — an identity that reached a backend's type renderer is an error,
    not output [backend-never-wrong].
  * **A slot is declared by a qualifier, a type declaration (`intrinsic` or
    `platform type`) or a struct** [struct-slot], and nowhere else. On a *fn*
    the binder binds bare in the signature instead (below), so a `?name:` in a
    fn's generics list is an error naming the two places it belongs: the
    parameter list, and the types the binder is written in.
  * [struct-slot] **A struct's slots** (user decision 2026-10-05, ROADMAP §0j
    step 6a; built 2026-10-05): `struct Ranked<T>(?cmp: (T, T) -> Int) canbe
    Mut { items: canbe Mut List<T> }`, in the generics or a `(…)` block after
    them, as a type declaration writes them. The identity is in the type and
    nowhere else: no value stores it, so the backends render nothing for it
    (the type renderers drop identities), and every fn over the struct takes
    it through its binder [cmp-binder], captured from the parameter's type
    (`fn push<T>(r: Mut Ranked<T>(?cmp), x: T)`). A literal adopts its
    position's identity like any type (`Mut Ranked<T>(?cmp) { … }`). The
    checker reads a struct's slot list wherever it reads a type's
    (`slotted_type`).
  * **Type arguments first, slots after** — one positional reading for a
    qualifier and a keyed type alike (user decision 2026-09-22):
    `Heap<Person>(cmp@Person) Mut List<Person>`, `SortedSet<Str>(my_cmp)`. A
    slot's type is instantiated by matching the qualifier's `of` type against the
    value, so `Heap<T>(?cmp: (T, T) -> Int) of List<T>` on a `List<Person>` wants
    `(Person, Person) -> Int`. An argument nobody wrote — a type argument or a
    slot — is **unconstrained**, which is what keeps every mention of one
    qualifier the same arity.
  * **A written slot list is a pattern, not an exact type.** A slot a signature
    does not mention is not constrained and the value keeps carrying it, so
    `Heap Mut List<T>` (nothing written) accepts any ordering and
    `Heap<T>(?eq) …` constrains the `eq` slot while saying nothing about `cmp`.
    `Heap<Person>(f)` as a *parameter* demands exactly `f`. That makes the bare
    form the empty case of one rule rather than a special case of its own.
    * **A `let` annotation is a pattern too** (2026-09-22, step 5): an
      unwritten argument is filled from the value, so
      `let live: Mut Sorted List<Int> = mut_sort(xs)` keeps the ordering the
      sort published and `add_sorted(live, 20)` can capture it. Written
      positions are demands as ever — an annotation naming *another* ordering
      fails the ordinary subtype check, naming both. Without this the one way
      to write a local's type would be to name an intrinsic `cmp` in it, and
      annotating would silently throw the identity away.
  * **A bare name is resolved where the type is used**, like any implicit
    [implicit-resolve]: `Heap<Person>(min_by_age)` names "the `min_by_age` visible
    here", and an *unwritten* slot is resolved by the slot's own name — which is
    why no default needs writing and why one slot means the right `cmp` at every
    instantiation. A `@`-scoped canonical is the spelling that
    does not depend on the reader's imports [fn-attached], and it is what a
    resolved identity is published as when the declaration has one.
* [cmp-binder] **The binder binds bare in the signature** (user decision
  2026-09-21, the ordering round's decision 11): all `?name` occurrences in one
  signature denote **one binding**. It is filled either way round:
  * by an **explicit implicit parameter** when the fn declares one —
    `empty_heap<T>(?cmp: (T, T) -> Int) -> +Heap(?cmp) Mut List<T>`, where
    [implicit-resolve] fills it and the *result type publishes what it chose*;
  * otherwise by **capture** from the argument types —
    `heap_push<T>(heap: Heap(?cmp) Mut List<T>, elem: T)`, where unifying the
    parameter against the argument binds it. Its fn type is never written at
    the fn: the slot it fills states it.
  * **Two occurrences must agree.** `merge(a: Heap(?cmp) Mut List<T>, b:
    Heap(?cmp) Mut List<T>)` forces both arguments to carry the same ordering,
    and a mismatch is the ordinary argument-does-not-fit error, which names
    both types (`Heap(by_name) Mut List<Person>` against `Heap(cmp@Person) …`).
    Two independent orderings are two names.
  * **In the body the binder is an ordinary implicit** [implicit-param]: a
    local of fn type, callable, forwarded to inner calls by name and type
    [implicit-forward]. What fills it at a call is the identity the *types*
    carry, before any resolution by name — which is how a heap is compared with
    the ordering it was built with rather than with whatever is visible at the
    call.
  * **A deduction that keeps the qualifier keeps the identity**, automatically:
    the identity lives in the type, and the fn could not have changed it.
  * **What fills a binder must have an identity.** `cmp = my_cmp` and
    `cmp = cmp@Person` do; a lambda does not, and passing one where the
    signature carries the binder is an error rather than a silently dropped
    claim.
  * **A binder names its slot, and may be aliased.** `?cmp` fills the slot
    `cmp` under that name; `?cmp: cmp2` fills it under the name `cmp2` — the
    destructuring spelling (`field: variable_name`), for the same reason: the
    left names the thing, the right names it here. Naming the slot is what lets a
    signature mention some slots and not others.
  * **An alias remembers its slot** (user decision 2026-09-22), so two
    candidates for one capability are an **ambiguity for the operator** whatever
    they are called: a signature holding two heaps ordered differently has two
    `cmp`s in scope, `a < b` there is refused naming both, and the body calls the
    one it means. Aliasing is how two orderings get into one scope, not how the
    choice between them is dodged — the operator asks what a parameter *is*, not
    what it is called.
  * Justification recorded with the decision: the bare binder is *an indirect
    way of declaring a fn in the parameter scope*, so it does not belong in the
    generics list — with the reservation that the generics-list spelling could
    be revisited if the bare form disappoints.
* [cmp-hash-values] A hash value holds **within one execution and nowhere
  else** (user decision 2026-09-21). Each backend hashes with its host's own
  algorithm — `hashCode()` on Kotlin, `DefaultHasher` on Rust — so the same
  value digests differently on the two targets, deliberately, on the analogy
  of the two backends' random numbers: the shape and the guarantees are
  identical, the values are not.
  * What is promised: `eq(a, b)` implies `hash(a) == hash(b)`, within one
    execution. Not promised: the converse (a collision is ordinary), stability
    across runs (a seeded hasher is within contract), or anything across
    backends.
  * So a program must not print, persist or transmit a hash value and expect
    it to mean anything elsewhere — an example's `expected.txt` cannot contain
    one, which is the posture `random` already has. A future reversal
    (language-defined algorithms for hash *and* random together) is recorded
    in ROADMAP.md, unscheduled.

### The filesystem (std, module `fs` and the three under it)

* [stream-layer] **Streams are a layer of their own** (user decisions
  2026-09-29, 20–31; built the same day). Module **`stream`** owns the linear
  tokens `InStream`/`OutStream`, `StreamError = InvalidUtf8 | StreamFailed`
  (each `{ source, … }` — a stream need not have a path), and **one effect,
  `Streams`**, with every stream operation: the reads (`read_line`,
  `read_all`, `read_bytes`, both `read_to`s, `read_line_to`), `position`, the
  writes, `flush`, and both `close`s — every consuming member beside the
  types, as the same-file rule [linear-group] requires. The iterators
  `Lines`/`Chunks` (`lines(s)`, `chunks(s, size)`), `fill_from` and
  `copy_stream` are `stream`'s too. Producers mint into the `Streams` in scope
  and never read: `fs` opens paths into it, a network client will hand out
  bodies the same way.
  * Why a layer and not a move: `InStream` alone could not leave `fs`, since
    `Fs.close` and a second effect's `close` cannot both be declared in the
    type's file — and a discharge grant or an exported terminal were the
    alternatives (the second makes linearity advisory). The layering puts every
    consuming member beside the type and needs no rule change.
  * `stream.host` is the `fs.host` shape: `RawStreams` (plain handles and
    errors), `threadsafe platform handler HostRawStreams`, and `DefaultStreams
    [RawStreams] of Streams`, which wraps failures in `Checked<StreamError>`
    and discharges the tokens in Salvo.
* [stream-table] **One host stream table per process, in Salvo** (moved
  2026-10-03, runtime step 14): the runtime service
  `runtime.streams` keeps the table — handle to entry, with the trap for a
  handle another provider minted [stream-provider] — and the read-ahead
  buffer, line splitting on `\n` and `\r\n`, the consumed-byte `position`
  (read-ahead counts only once handed out), failure recording (a failed read
  reports the end and the close reports why; writes likewise at flush and
  close) and strict UTF-8. An operation **checks the entry out**, works with
  no lock held, and checks it back in, so a slow read of one stream never
  blocks another. The host supplies `linear platform type HostIn`/`HostOut`
  and five leaf fns (read a chunk, write, flush, close, a stream over bytes).
  `stream.host`'s `HostRawStreams` is an ordinary Salvo handler over it, and
  `raw_receive` reads on a runtime thread, answering through a task.
  `stream.fresh_handle` is Salvo too. Host code's entry points stay
  (`hoststreams.rs`/`.kt`, shipped wherever `runtime.streams` is):
  `salvo_stream_register_in/out` / `SalvoStreams.registerIn/Out` take a
  source and an `io::Read`/`InputStream` (or the write side) and answer a
  handle, and `salvo_stream_take_in` / `SalvoStreams.takeIn` take a stream
  out for host code to read itself (the S3 glue's upload), read-ahead first.
  `HostRawFs` registers what it opens.
* [stream-values] **Numbers and values on a stream** (user decision
  2026-10-04): `write_int`/`write_long` and `read_int`/`read_long` (4 and 8
  bytes, big-endian, two's complement), and `write_value(out, v)` /
  `read_value<T>(in)`: the canonical encoding [wire-format] behind a 4-byte
  length. Ordinary Salvo over `write_bytes` and `read_bytes`, so a fake
  `Streams` serves them. A read answers `Ok …`, `End` when the stream ends
  before the first byte, or `Err Checked<StreamError>` when it ends inside a
  number or value, or the bytes do not decode as the type asked for. The
  codec is an implicit (`?encode: (v: T) -> Bytes`, `?decode: (data: Bytes)
  -> T?`), filled at the caller's concrete type by `codec`'s `encode`/`decode`
  — which the caller imports — since a wire form is the instantiation's and a
  generic body may not name it [noremote]. Both backends lower `encode`/
  `decode` as a value at the type of the position they fill, as they do for
  `protocol`.
* [stream-receive] **`Streams.receive(s, reply: Reply<Received>)` reads
  without blocking** (§4b step 3d, 2026-09-29): it consumes the stream and the
  answer hands it back — `Received = Ok Packet | End | Err
  Checked<StreamError>`, `linear struct Packet { bytes, stream }` — so one read
  is in flight, and a `close` mid-read cannot be written because the token is
  not there to write it with. At `End` or a failure the provider has already
  closed the stream, so nothing is owed. A `Packet` is taken apart with
  `let {bytes, stream} = packet`, and `close(Packet)` gives up the stream
  inside. Chunks are provider-sized (64 KiB on the host and in memory) and
  never empty (decision 8).
  * Host: `DefaultStreams.receive` discharges the token, hands the handle to
    `RawStreams.raw_receive`, and mints it back in `host_received`, the
    continuation the host completes. `HostRawStreams.raw_receive` reads on a
    thread of its own and completes the reply from there [platform-reply], so
    no Salvo worker waits. At `End` or a failure it releases the stream —
    out of the table *and closed*, which on Kotlin must be said: a JVM stream
    is not closed by being dropped (fixed 2026-09-30; an S3 body's response
    block waits for exactly that close).
  * Memory: `MemFs.receive` answers at once (still as a later activation).
* [stream-from-bytes] **`Streams.from_bytes(data) -> InStream`** mints a
  stream over bytes in hand, in the table of the `Streams` in scope — the host
  table (`RawStreams.raw_from_bytes`, an `io::Cursor` / `ByteArrayInputStream`)
  or `MemFs`'s. A request body a program built, a test's fixture.
* [stream-pipe] **`pipe(from, to, done: Reply<Ok Long | Err
  Checked<StreamError>>) [Streams]`** copies without blocking a worker: a
  chain of `receive`s through the free `send fn pipe_step`, on the pool it was
  called on; both streams are closed at the end and the first failure is
  reported. With [stream-table]'s one host table this is PutObject from a file
  and GetObject to a file with no adapter — the shape `examples`-level tests
  exercise on both backends.
* [stream-provider] **A stream belongs to the provider that minted it.** A
  handle from another table — a `MemFs` stream read by the host's `Streams`,
  or the reverse — is a program bug and **traps**, naming the rule (user
  decision 2026-09-29, 10); `StaleHandle` is gone from the error kinds with
  it. With one handle counter the wrong table always finds an unknown handle,
  never another live stream. Making it a compile-time error is ROADMAP §4b
  item 4.
* [fs-surface] Module **`fs`** declares the **path surface**: an `FsError`
  union — `NotFound | PermissionDenied | AlreadyExists | NotADirectory |
  PathEscapes | IoError | Streaming` — carried by `Checked<FsError>`
  [checked-type], `FileInfo`, **`effect Fs [Streams]`** (the path operations;
  the opens mint into the `Streams` in scope [effect-prereq]), `open_lines`,
  `open_chunks`, and the one-shots (`read_to_str`, `read_lines`, `write_str`,
  `read_to_bytes`, `write_bytes_to`, `copy_file`). Application code declares
  `[Fs]` and nothing else: the prerequisite brings `Streams`.
  * **`Streaming { error: StreamError }`** is how a one-shot that opens *and*
    reads reports a read failure through one type. It wraps rather than
    merging `stream`'s kinds into the union, so a value of either type has
    exactly one text form [interp-to-str] — with the kinds merged, an
    interpolated `StreamError` found two `to_str`s and interpolated as neither.
  * Fallible members return `Ok T | Err Checked<FsError>`: an effect member may
    declare no effects, so there is no `[Throw]` here
    [effect-member-no-effects]. The `Err` arm is linear, so a result that
    is never looked at is a compile error, and narrowing to `Ok` discharges
    it [linear-union-arm]. The dischargers are `Checked<T>`'s own —
    `ignore(e)` acknowledges, `detach(e)` hands back the bare `FsError`,
    which is what an aggregation collects, since a list of *errors* wants no
    obligations in it even now that a container could hold them
    [linear-container]. There is no reading form that keeps the obligation:
    looking at a failure means dealing with it, so a rendering is
    `to_str(detach(e))` (user decision 2026-09-26: one mechanism for "you
    must look at this", not two).
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
  * A token handed to a handler that did not mint it traps
    [stream-provider].
* [fs-errors-at-close] Read and write errors are **recorded** by the
  handler and surface at `close` (and `flush`): `read_line` reports the end
  of the stream either way, and `write` returns only the byte count, so a
  loop never narrows a result per line. `close` on both token types returns
  `Ok None | Err Checked<StreamError>`, so dropping it on the floor does not
  compile. (The stream members are `Streams`' since [stream-layer]; the rules
  kept their `fs-` labels.)
* [fs-bytes] Bytes are part of the v1 surface: `read_bytes(s, max) ->
  Ok List<Byte> | Err Checked<FsError>` answers **up to** `max` bytes (fewer means
  the stream ended, none means it had ended already) and
  `write_bytes(s, data) -> Long` writes them, neither encoding nor decoding
  anything — a file that is not text is read and written by the same effect.
  * **One stream, one position, counted in bytes**: text and byte
    operations interleave on a stream, so a `read_all` continues exactly
    where a `read_bytes` stopped. This is why the host buffers *bytes*
    below the decoder (the runtime's stream table [stream-table]) rather
    than reusing a character-counting reader.
  * A ranged open that lands mid-codepoint is a **legal seek** — an offset
    is bytes, and bytes have no characters. The strict decode afterwards is
    what fails (`Err InvalidUtf8`), and the failure is recorded, so `close`
    reports it a second time [fs-errors-at-close].
  * The payload type is **`Bytes`** [bytes-type] — `Vec<u8>` on Rust, the
    shipped buffer class on Kotlin [kt-bytes].
* [fs-read-to] Every read has a **fill-a-buffer** form, for the loop where a
  payload per step is the cost (user decision 2026-09-15): `read_to(s, buf:
  Mut Bytes, max) -> Ok Int | Err Checked<FsError>`, `read_to(s, buf: Mut Str) ->
  Ok Long | Err Checked<FsError>` (the `read_all` parallel), and `read_line_to(s, buf:
  Mut Str) -> Bool` (the `read_line` parallel — `false` for end-of-stream or a
  recorded failure, exactly as `read_line` answers `None`). One name for the
  two `read_to`s: the **buffer's type** picks the overload
  [effect-member-overload].
  * They **append**, never overwrite, so `size(buf)` is the data and no
    "only the first n are meaningful" convention exists; `clear(buf)` between
    steps is what makes one buffer serve a loop. It is also what lets `MemFs`
    implement them with `append` alone [fs-double].
  * `RawStreams`' mirror splits the names (`raw_read_to_bytes`,
    `raw_read_to_str`, `raw_read_line_to_str`) rather than overloading: the
    host file is *hand-written*, and an overload set would make it implement
    mangled names [fs-host-split].
  * Riding along: `chunks(s, size)`/`open_chunks` — an iterator over a stream's
    bytes as `Lines` is over its lines, **a fresh buffer per step** (an iterator
    recycling its own would overwrite what the caller holds) — and the
    one-shots that keep the buffer inside std: `copy_stream(s, w)`,
    `copy_file(from, to)`, `read_to_bytes(path)`,
    `write_bytes_to(path, data)`, `fill_from(s, buf)`.
* [fs-module] The filesystem is **imported, not implicit** (user decision
  2026-09-26): module `fs` holds the surface, and `fs.host`, `fs.mem` and
  `fs.restricted` hold the implementations. A whole-module import names one
  module [mod-import-module], so `import fs` is the surface alone and each
  implementation is its own line — which is the whole point of the split: a
  test that fakes the filesystem never mentions the host's, and a program that
  never opens a file links none of it.
  * It left `core` because nothing in `core` needs it and everything in `core`
    is visible everywhere. `core.fs` used to be dragged into any program that
    iterates or interpolates (it declares a `next` and a `to_str`), which is
    what [mod-used-only]'s name-based half does; an imported module cannot be.
    Measured on `examples/throw-and-release`, which throws and never opens a
    file: it stopped emitting `core/fs`, `core/bytes` (reached only through it)
    and 136 lines of union wrappers — **1,398 lines** of generated code across
    the two backends, for a program whose behaviour did not change.
* [fs-host-split] The host-backed filesystem is its own module, **`fs.host`**:
  `effect RawFs` (the path operations and the opens; plain `Long` handles into
  the host stream table [stream-table], bare `FsError`s),
  `platform handler HostRawFs of RawFs` (std ships
  `std/platform/fs/host.{kt,rs}` [platform-handler]) and
  `handler DefaultFs [RawFs] of Fs`, which mints the tokens, wraps failures
  in `Checked<FsError>` and discharges in Salvo — the host never holds an
  obligation.
  * The split was load-bearing while the fused emission existed (a dependent
    handler switched the whole program to it); since 2026-09-28 the emission
    is one shape and the split is a matter of what a module drags in.
  * `[RawFs]` stays greppable as the audit: nothing but a composition root
    (`use HostRawFs()`) and `DefaultFs` reaches raw handles.
* [fs-double] `fs.mem` ships **`MemFs of Fs, Streams`**: an in-memory
  filesystem *and* stream table in pure Salvo, with no dependency and no host
  anywhere, so a test that registers it touches no disk. One handler wearing
  both faces (user decision 2026-09-29, 29), because a write stream must
  publish into the file map on close, and under [effect-prereq] a handler that
  wears the prerequisite depends on nothing for it. A read stream reads a
  snapshot of the file's bytes taken at the open. No separate `MemStreams`
  until a test with no filesystem needs one.
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
* [fs-restricted] `fs.restricted` ships **`RestrictedFs(root: Str) [Fs]
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
  * The policy lives entirely in the **opens**, and it intercepts `Fs` only:
    what they open is read by the `Streams` in scope as usual.
* [fs-v1-cuts] Not in v1, and each an error rather than a surprise: seek
  (a ranged `open_read_at` replaces it, so streams stay forward-only),
  recursive walk or delete, temp files, watching, permissions, symlink
  creation, and stdin (Console's, not Fs's). `rename_path` is spelled with
  the suffix because `rename` is a keyword [fn-rename].

### Non-resumption: `throw` and `try`

* [throw] `throw(message)` leaves the enclosing delimiter instead of
  resuming. It is declared in std (module `throw`, imported rather than
  implicit — `import throw`, so a program that never throws links none of it;
  a **test annex** gets it without writing one, because the harness and not
  the author puts `[Throw<Failure>]` on a test body [test-implicit-import],
  2026-09-26) as the sole member of
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
    authority, so `throw`'s `thrown(message)` constructor produces a
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
does not exist yet. Nothing here is in docs/language/ until the feature runs;
docs/language/ remains the source of truth for everything that does.

* [actor-kind] An **actor** is a handler whose members run one at a time,
  on a scheduler, in the order their invocations arrived: a state struct plus
  one function per member, exactly the handler that `use` binds
  synchronously. The same handler is bindable both ways — the binding
  changes where the body runs and nothing about the code that calls it.
  * The substrate is **std's `runtime` module, written once in Salvo**
    [runtime-sched] (built 2026-09-15 as a library in each backend's runtime
    files; ported 2026-10-03), not a runtime baked into emitted code:
    run-to-completion activations on pools, one arrival-order queue per actor
    with an explicit bound, replies with reserved capacity, the gate, death as
    a faulted activation, and the idle-with-parked-gates report.
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
  ([effect-handle]) **a `use H(args)` of a stateful plain handler is also a
  sharing site** — the same lock shape, minted at the binding — so the
  spawn spelling and the `use` answer one form.
  * **The dependency restriction is lifted** (user decision 2026-09-20;
    it was "a monitor-spawned handler declares no dependencies"). A monitor
    may declare dependencies when every one is the shareable default —
    captured as owned handles at construction ([effect-handler-deps]),
    resolved from the enclosing scope exactly as a `use` resolves them. The
    availability rule keeps the capture acyclic: a dep was bound before
    this handler, so no lock order can cycle and no path routes back —
    which is what keeps the JVM-reentrant/Rust-non-reentrant parity trap
    closed [backend-never-wrong], *provided the door stays shut*: no `use`,
    no `spawn` capability on a shared handler (bindings are fixed at
    construction).
  * **Waits under the lock are priced, not refused** — option (b), user
    decision 2026-09-20, chosen over a wait-free restriction since the
    deadlock graph already exists: a dep-bearing or waiting shareable plain
    handler gets a graph node (`H's lock`), occupancy-style edges in (from
    handlers depending on its effect) and out (to mixed servants and priced
    monitors of its dep effects; `waitfor` in members as blocks), so a
    cycle through a held lock is reported before it runs, naming the locks.
  * **Refused with it, each by name**: an `on` clause (members run on the
    callers' threads — there is nothing to place); more than one plain
    face on a *spawn* (the spawn answers one `Addr<E>`; a `use` of the same
    handler binds every face over one lock, [effect-handle]); a `proj`-holding
    constructor parameter or state field [actor-sendable] (the instance
    crosses to every thread that binds the handle — a stored *function
    value* is fine since 2026-09-28, both backends storing it in a form that
    crosses); and a `mailbox` slot, refused at the declaration already (a
    plain-face handler has no queue to bound [actor-mailbox]).
  * **Serialization without a servant**: members are mutually excluded by the
    lock, not serialized by a mailbox — two handle holders' calls interleave
    per member, and there is no arrival order, no gate, no death, nothing to
    `watch`. A member calling a sibling member runs within one acquisition on
    both backends (a direct self-call underneath).
  * Calling a plain member *through the addr* (`rng.next()` without a `use`)
    stays refused for now, with the send-member diagnostic; `use` the handle.
  * Lowering: [rs-handle] and [kt-monitor] — a per-effect lock wrapper
    (`__Handle_E` / `__Mon_E`) implementing the effect's trait/interface by
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
  * A `send fn` **no face declares** is a private member of the handler —
    [actor-private-send].
* [actor-msg] **The messages of an actor are declarations the compiler
  writes** (IR step 4, 2026-10-06): for an actor effect `E`, an enum
  `__Msg_E` with one variant per `send` member (carrying its arguments and,
  when the protocol has a wire form, its hash); for an actor handler `H`,
  `__Priv_H` (`Init`, and one variant per private `send fn`) and `__Cont_H`
  (one variant per member that parks on a reply, holding what it had
  captured). Not source, never named by a program; every backend renders the
  same enums, and the wire codec of `__Msg_E` is the backend's.
* [actor-dispatch] **Delivering a message is a generated function**:
  `__dispatch_H_E(handler, msg)` per face and `__dispatch_priv_H` for the
  private enum, each a switch on the message whose arm unpacks the variant's
  payload and calls the handler's member directly (not through its handle).
  The scheduler glue that receives a frame and calls it is the backend's.
* [actor-private-send] **A handler of an actor effect may declare `send fn`
  members no face declares; they are private** (user decision 2026-09-27).
  A private member has no message in any protocol, so nothing outside the
  handler can send to it: the only ways to reach it are `k@self(…)` from one
  of the handler's own members [actor-self-send] and `replyto k(…)`
  [actor-replyto], both aimed at **this instance**. A private member with at
  least one parameter may be a continuation target, its trailing parameter
  being the answer. Private members are not part of the protocol hash
  [protocol-hash] and never cross the wire as messages; their continuations
  are still answered from another node, since a reply travels by slot.
  * **It carries the free send fn's obligations** [free-send-fn], because no
    effect declaration mirrors it: a written, all-consumed deduction clause
    (`=> !label, !out, !total`), no `Mut` parameter, no generics, sendable
    payloads — the same rule a mixed handler's servant members have always
    had [mixed-handler], which are this rule's oldest case. Its own clause is
    its linear-discharge context [linear-group].
  * A **bare call** to a private member resolves to nothing (it is a message,
    not a function); the diagnostic names the selector: "`k` is a send member
    of `H`, so it is sent, not called: `k@self(…)`".
  * Emission: a handler-keyed private message enum (`__Priv_H`) beside the
    faces' `__Msg_E`s — separate because a face enum is the protocol's wire
    form and hash — one more downcast arm in `handle`, direct-call arms in
    `resume`, the member body as an inherent method (a dependent handler's
    through its `__Impl_H` trait). Until 2026-09-27 such a member type-checked
    and had no variant to arrive on (a raw rustc error); a stop-gap refusal
    that day was replaced by this rule the same day.
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
* [handler-init] **A handler may declare one `init { … }` block, which runs
  once, first** (user decision 2026-09-27). Contextual like `mailbox` — a
  state field named `init` is `init: T = …`; a brace makes it the block — and
  checked as a parameterless **private send member** named `init`
  [actor-private-send]: it sees constructor parameters, initialised state
  (and may assign it, which gives a handler a constructor body), and the
  handler's dependencies; it declares no effects; its sends count as the
  handler's in the deadlock graph. A written member named `init` beside the
  block is refused; a mixed handler has no `init` yet (its servant starts
  with a send member).
  * **Spawned**, `init` is the actor's **first activation**: the spawn
    enqueues it before handing back the address, so no message anyone sends
    afterwards can overtake it, and the address is already written when it
    runs. **`use`-bound**, it runs inline right after construction, on the
    caller's thread — a dependent handler's after the fused value that
    carries its dependencies exists [rs-actor].
  * **`self@Face`** is the enclosing handler's own address as one of its
    actor faces, an `Addr<Face>` — legal in `init` and in send members,
    refused outside a handler, for a face the handler lacks, and for a plain
    face (a monitor has no address). A handler that names its address is
    **spawn-only**, reported at a `use` of it exactly as a `replyto` handler
    is [actor-replyto]: a `use`-bound instance has no address to name. With
    several faces the value is the same actor under each face's type.
  * What it replaced: three "hand me my own address" handshakes in std `net`
    — `NodeGroup.join(events)` with the `node_group()` helper, `ActorGroup.
    start(me)`, and the opener subscribing the replica to the node group —
    each a member whose only job was to receive the address a spawn
    answered. A mechanism now starts itself; `PeerEvents` stopped being a
    face; a spawn of a node group answers one `Addr<NodeGroup>`.
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
* [pool-retire] **A `Dedicated` pool's thread ends with its actor** (2026-10-03,
  runtime step 13): `thread()` makes a pool nobody else can hold —
  the `on` clause consumes it — so once every actor on it is dead, no task is
  queued there and no token is owed to work there, nothing can run on it
  again. The core marks it retired, wakes its thread, and the thread returns.
  An ordinary `pool(n)` stays: its value may still be held and spawned on.
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
  * **No generics**: a mint carries captures and no type arguments, so there
    would be nothing to choose an instantiation by.
* [task-effects] **A task body's effects are inherited from the frame that
  mints it** (user decision 2026-09-26). A free `send fn` declares them
  ordinarily (`[Console]`), and the **mint** — not the declaration — is where
  they are supplied: `replyto report(out)` resolves each against the minting
  scope and hands the task a handle. The first-pass cut was "no effects but
  `[waitfor]`", written before there were thread-shareable handlers; there are
  now, and this is the same inheritance `spawn` carries into a child
  [spawn-inherit].
  * **Resolution is the mint's, and it is the checker's** — one implementation,
    reused: the exact instance if the scope has it, otherwise the single
    compatible one, with an ambiguity refused rather than guessed. Three
    failures, each naming its own remedy: nothing in scope (bind a handler
    before the mint, or declare the effect on the minting function so it is
    supplied from further out), several candidates, and — transitionally,
    ROADMAP §2b — an **inline** binding the emitters cannot yet hand a body
    that runs after the scope ends.
  * **Two forms still cannot be declared on a task**, because they are
    capabilities of a *frame* rather than of a body: `[use]` registers handlers
    for the rest of a scope and a scheduled body has no scope anything else can
    see, and `[spawn]` places work on a pool from the frame that has one — a
    task already *is* that work. `[waitfor]` is unchanged.
  * An `Addr` capture remains the way to reach an **actor** from a task, and
    needs no effect declaration [actor-use-addr].
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
    marker (without it, `use H(args)` is the in-scope handle binding); a
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
    *remote mint* — the generalized form (decided; ROADMAP.md's sugar iterator) — and
    is refused for now with the workaround that needs nothing new: a token is
    an ordinary linear value, so the handler that owns `k` mints it and iterators
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
    system's word and a platform handler wrapping process management will want
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
      (the sugar iterator's remote mint reserves in *another* actor's queue, so
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
  * **The runtime builds it**: `watch` is Salvo (`core.actor`), taking the
    core's token out of the reply token (`reply_token` [runtime-handles]) and
    registering it with `runtime.watch`; the scheduler, Salvo too, answers it
    with `erase(Exit { reason })` — so a watcher's payload is an ordinary
    answer, identical to `r.send(Exit{…})`. Until 2026-10-06 the watch site
    handed the scheduler a constructor it no longer used.
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
    effect that does not wait (a monitor, or a non-mixed handler). Over
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
  plain std struct with a single `nanos: Long` field,
  `: Ordered<self> by auto, Hashed<self> by auto` [obligation-by].
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
    }`) over `runtime.after_nanos(delay, done: Reply<Long>)`, a private
    continuation turning the reading into the `Fired` its caller is owed. Its
    mailbox bounds *registrations*, not deadlines.
  * **The deadlines are Salvo too** (2026-10-02, runtime step 11 slice
    11a): the service `runtime.timers` [runtime-layers] keeps them in a `Deadlines` monitor and serves
    them with **one wheel** — an actor on a dedicated thread, bound by the
    module's `use Wheeling() on thread()` [mod-use], whose one activation
    fires what is due and parks on its `Parker` [runtime-parker] until the
    earliest deadline left, returning when none is. No thread per timer, no
    polling, and no timer machinery in either host scheduler. A pending
    deadline counts as work on its way because the wheel's frame is
    *running* while one is, which holds off both the quiescence hook
    [actor-on-idle] and the deadlock report; with none pending the wheel has
    no frame. A registration wakes the wheel by unparking it, and the park's
    token closes the window between "what is earliest" and sleeping.
  * One difference from the host timer it replaced: a token handed to
    `after_nanos` stays tracked until it fires (the old runtime untracked it),
    which is never visible, since `on_idle` cannot fire while a deadline is
    pending.
* [time-manual] **`handler ManualTime() of Timer, TimerCtl`** is the
  pure-Salvo fake — `MemFs`'s answer applied to time [effect-handler-multi].
  Virtual time starts at zero and moves only through
  `TimerCtl.advance(by)`, firing every deadline it iterators **in deadline
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

## Network (std, module `net`) — step ① of the network sequence

The wire under actors across machines (user decisions 2026-09-26; ROADMAP.md
section 2). The design's one line: **the network enters at the actor group,
never at the spawn** — every actor is spawned by the node that hosts it, and
what crosses the wire is addresses and messages. This module is the bottom of
that stack and knows nothing about groups, nodes or protocols: it moves frames
between endpoints and delivers what arrives into the scheduler.

* [net-transport] `effect Transport` is the wire, a **plain** effect whose
  members are calls on the caller's thread: `listen(at: NodeEndpoint, sink:
  Addr<Inbound>)`, `unlisten(at)`, `deliver(to: NodeEndpoint, frame: Bytes) ->
  Ok None | Err NetError`, `local_endpoint()`. Frames are opaque `Bytes` — the
  codecs (step ②) sit one level up. `NetError = Unreachable | WireFailed`.
  * **Delivery is at most once and in order per (sender, receiver) pair**, and
    nothing more: `Ok` from `deliver` means the transport *accepted* the frame
    (written to a connected socket), never that the far side has it.
    Anything stronger is a protocol over this, not a promise of it.
  * **Inbound is an actor effect**, `actor effect Inbound { send fn receive_frame(from:
    NodeEndpoint, frame: Bytes) }`, because bytes arrive on a thread the
    scheduler does not own and a send is the only door in [actor-effect-kind].
    A node that wants to receive spawns an actor serving `Inbound` and hands
    its addr to `listen`; the host forwards every arriving frame through the
    generated stub (`__Stub_Inbound` / the Kotlin twin) — the same
    host→runtime upcall the timer makes [time-timer], and nothing new: an
    `Addr` is a scheduler index the host can hold, and the stub already
    exists per actor effect [actor-use-addr].
  * `NodeEndpoint { host: Str, port: Int }` is where a transport can dial,
    known **before** any contact — hashable and ordered, so it keys maps and
    sorts in a membership list. Distinct from the `Node` step ④ mints *after*
    a handshake.
* [net-host] `threadsafe platform handler HostTcpTransport(bind: NodeEndpoint)
  of Transport` is std's first `threadsafe` host [threadsafe-platform] — a
  transport is reached from every pool at once — with one class per backend
  in `std/platform/net.{rs,kt}`, both signing the contract. The wire is
  identical on both: one TCP connection per peer, opened lazily by the first
  `deliver`; it opens with a **hello** (the sender's endpoint) and then carries
  frames, each a 4-byte big-endian length and that many bytes. `listen` runs
  one acceptor thread per endpoint and one reader per connection; a failed
  connection is dropped and the next `deliver` reconnects.
  * **An open listener is registered with the scheduler as an outside source
    of work** (`salvo_external_begin/end`, `SalvoSched.externalBegin/End`):
    while one is open the program is neither quiescent nor deadlocked, exactly
    as with a pending timer — a frame may still arrive from a thread the
    scheduler does not run. This is [actor-on-idle]'s "a platform handler with
    a thread of its own can make idle stale" caveat made a fact the runtime is
    told rather than guesses. Found the moment the first TCP smoke test ran:
    without it the Rust runtime declared `main`'s wait a deadlock while the
    frame was in flight on the socket.
* [net-mem] The double, in two pieces mirroring what a network is: **one
  network many nodes share** and **one transport per node** over it. `actor
  effect MemNet` (handler `MemNetwork`) holds the listener table and the
  faults a test scripts — `partition(a, b)` / `heal(a, b)` sever and restore a
  pair, `kill(node)` takes a node off until something `attach`es there again,
  `delivered(out)` counts routed frames; `route(from, to, out: Reply<Addr<
  Inbound>?>)` answers the listener or `None`. `handler MemTransport(me:
  NodeEndpoint, net: Addr<MemNet>) of Transport` is **stateless** — the
  network holds everything — so a `use` binds it bare and shareable; its
  `deliver` waits on the network for the route (a `waitfor` on the caller's
  thread, serving the caller's pool meanwhile [waitfor-pump]) and sends to
  the listener. It runs the same `Inbound` actors, frames and `Err`s the real
  transport does; what it does not do is serialize, so a codec bug is
  invisible here until step ② encodes on both sides of `deliver`.
  * Why two pieces rather than one handler with a fault face: a handler of
    *several plain effects* cannot yet be shared stateful (one lock behind
    several faces has no backend form — the monitor spawn's recorded gap), so
    the controls could not ride on `MemTransport` as a second face the way
    `ManualTime` carries `TimerCtl` (actor faces are free). The split also
    reads better: faults belong to the network, not to one node.
  * Test bodies have no `spawn` [test-body], so `net` is covered by a
    compile-and-run case per backend (identical output) rather than a
    `.test.sv` annex until the actor-testing slice lands.
* [wire-format] **The canonical encoding** (step ②, user decision 2026-09-26:
  Salvo owns serialization, so a Kotlin node and a Rust node share a group).
  Stated once, in `salvo-core`'s `wire.rs`, and implemented twice, in each
  backend's `wire` runtime, byte for byte; struct, union and message codecs
  are **generated** by the emitters beside their declarations.
  * `Bool` one byte; `Byte` one; `Int` four bytes big-endian; `Long` eight;
    `Float` four (IEEE-754 bits); `Double` eight; `Char` four (the code
    point); `Str` a `u32` byte length then UTF-8; `Bytes` a `u32` length then
    the bytes; `None` nothing. An optional (`T?`, or any union with a `None`
    arm) is a presence byte then the payload. A union of *n* non-`None` arms
    is one tag byte holding the arm's index over the **declared** arms in
    declaration order — the positional identity checker and emitters already
    share — then the payload. A struct is its fields in declaration order; a
    tuple its components; a `List`/array a `u32` count then the elements;
    qualifiers erase [qual-erasure]. An actor message (`__Msg_E`) is a union
    of the `send fn` members in declaration order, each its parameters.
  * **Decoding is total**: a malformed input — truncated, a tag out of
    range, invalid UTF-8, trailing bytes — answers `None`, never a trap.
  * The surface is std `codec`'s (`net`'s until 2026-10-05) `encode<T>(value: T) -> Bytes` and
    `decode<T>(data: Bytes) -> T?` (written `decode<Point>(data)`, since
    only the type argument says what to read), both **refused at the call**
    for a type with no wire form, naming what stops it. A generic `T` is
    refused too: the form is the instantiation's, and `encode` inside a
    generic body has nothing to dispatch on (recorded cut; a `Wire` bound
    would lift it).
  * **Keyed containers** (`Set`, `Map`, `SortedSet`, `SortedMap`) have no
    wire form yet — their identity capabilities (`hash`/`eq`, `cmp`) are
    resolved by the checker per type and would have to travel into the
    decoder; send a `List` of the entries. Recorded cut. `Addr<E>` and
    `Reply<T>` wait for step ③'s routable identity.
* [noremote] **Serializable by default, `noremote` the opt-out** (user
  decision 2026-09-26, refusing the opt-in marker as the Akka experience —
  forgotten at the leaf, reported at the root). `noremote struct S { … }`,
  `noremote intrinsic type T`, `noremote type A = …`: the type has no wire
  form, and **neither does anything holding one** — a struct with a
  `noremote` field is `noremote` whether or not it says so, and the
  diagnostic names the field (`Frame.canvas (Canvas)`). Contextual, like
  `actor`; before anything but a type declaration it is an error saying what
  it is for. A function value and a `proj` view are `noremote` by
  construction. std marks its process-local handles: `Pool`, `InStream`,
  `OutStream`.
  * **One predicate, two consumers** (`salvo_core::wire_blocker`): the
    checker refuses `encode`/`decode` with it, and both emitters generate a
    codec for exactly the structs that pass it — so a value that encodes has
    a codec on both backends, and the two can never disagree
    [backend-never-wrong]. From step ⑤ the same predicate refuses
    `actor_group<E>` of a protocol with a blocked payload — the crossing site the
    user chose over the declaration.
* [protocol-hash] **The canonical hash of an actor protocol**: FNV-1a 64
  over the canonical form — every `send fn` in declaration order as
  `name(types);`, each struct **expanded to its field types** (so a renamed
  struct is the same protocol and a reordered field is not, matching the
  positional encoding), unions as their arms, qualifiers erased — rendered as
  sixteen hex digits and emitted as a constant beside the message codec
  (`__PROTO_E` on both backends). Computed by the compiler
  (`Checked.protocol_hashes`), so the two backends carry the same constant
  and no runtime hashes anything. What the handshake exchanges (step ④) and
  `actor_group`/`join` compare, **never at decode**: exhaustive `when`
  over positional arms means an old node has no value to construct for an
  arm it lacks, so compatibility is settled before a byte is read.
  * Generated only for a protocol whose every payload has a wire form; one
    with a `noremote` payload is a legal *local* protocol with no hash.
  * **As a value**: `struct Protocol<E> { name: Str, hash: Str }` and
    `intrinsic fn protocol<E>() -> Protocol<E>`, std `net`. The intrinsic is
    the one lowering that reads its type argument — the constants exist only
    where `E` is written concretely, since an effect-only generic is erased
    [effect-generic-decl] — and its job is to be what the checker finds for
    an implicit `?protocol: () -> Protocol<E>` [implicit-resolve], so a fn
    generic over the protocol (`actor_group<E>`, or a program's own) holds the
    constants as a value it was handed [actor-group]. The name and the hash
    are the struct's fields; there are no separate accessors (the private
    `protocol_name`/`protocol_hash` were deleted 2026-09-28, unused).
* [addr-routable] **An `Addr<E>` is a routable identity** (step ③, user
  decisions 2026-09-26): in generated code still a scheduler index, but every
  entry carries the **node** it lives on and its **bits**, so its wire form is
  `(node, actor, bits)` — 24 bytes — and a `Reply<T>` crosses as `(node,
  kind, id, slot, bits)`. **Locality is a runtime fact, read at the send**: an
  addr decoded from the wire whose node is not the current node becomes a
  **proxy** — an entry with no body — and a send to it is a MSG frame; a
  reply token that crossed answers with a REPLY frame; a function holding the
  addr never learns which. The same `send`/`waitfor`/`replyto` code runs
  unchanged. `Addr<E>` has a wire form exactly when `E` has one (a proxy is
  only good for sends that can be framed), `Reply<T>` when `T` has one.
  * **The routing is the service `runtime.routing`** [runtime-layers]
    (2026-10-03, runtime step 11): identities, proxies, credits,
    routes and the frames, written once in Salvo. Frames are one union,
    `RtMsgFrame | RtAnswerFrame | RtGrantFrame | RtOpenFrame |
    RtControlFrame`, in the canonical encoding [wire-format] (D4), so no
    hand-written frame code is left in either host. An actor's capability
    bits are minted the first time its identity is asked for, which is the
    first time its addr leaves its node; an identity claiming bits for an
    actor that never left is the shared dead entry. The core marks a proxy's
    entry, so a send to it is handed back to the typed send, which encodes it
    and passes it on; a local send never consults the routing table. What
    stays with each host is what only generated code knows, reached through
    the service's platform fns: each actor's message decoder, each waiting
    frame's and exported task's answer decoder, each control sink's message
    builder, and each node's outbound hook. The route-stub views and
    `key_hash` stay host-side until step 12.
  * **`NodeId`** (2026-09-27) is the identity of a node as a Salvo struct
    (`struct NodeId : Hashed<self> by auto { id: Long }`), answered by
    `this_node()`/`new_node()`/`node_of(addr)` and taken by `pool_at`,
    `Node.id`, `Leader.leader() -> NodeId?` and the mechanisms' members. It
    has a wire form because it is compared across machines; a raw `Long` no
    longer types as a node, so a pool cannot be placed on an arbitrary
    number. The emitters build it around the runtime's `u64`/`Long` at every
    intrinsic that answers a node and read `.id` at every one that takes one.
  * **A process is one node with one random identity, and may host further
    virtual nodes** — `new_node()` and `pool_at(node, n)` in std `net` — so a
    program runs several nodes in one process over `MemTransport` and every
    remote path (proxies, credits, frames, remote replies) runs without a
    socket. An addr of a hosted node other than the current one is a proxy in
    every respect but the socket; `this_node()` answers the current pool's.
  * **The runtime's outbound is an actor**: `route_frames(out: Addr<Outbound>)`
    (Salvo since 2026-10-06: it hands the host the named fn `forward_frame`
    through `bind_outbound` [platform-fn-value], which decodes the endpoint
    and sends `send_frame` to the actor) binds the current node's wire (`Sending [Transport] of Outbound` in std
    hands frames to the transport in scope), `add_route(node, at)` says where
    a node's frames go, and `Receiving of Inbound` hands arriving frames to
    `deliver_frame`. A frame for an unknown target, with mismatched bits or a
    malformed payload is **dropped, never delivered wrong**.
  * [net-connect] **`connect(me) -> Bool`** (2026-09-27) is the three bindings
    in one call — the outbound actor bound with `route_frames`, the inbound
    actor registered with `listen(me, …)`, the self-route `add_route(this_node(),
    me)` — and the one a program writes; `connect(me, on: Pool)` places the
    two actors (default `pool(1)`). It is **idempotent**: `connected()` answers
    whether the current node's outbound side is bound, and `connect` answers
    whether *this call* did the binding (`false` when it was already
    connected — nothing is rebound, nothing spawned). **A node group's `init`
    connects the node when nothing has** (user decision 2026-09-28): it calls
    `connect(local_endpoint())`, the transport's own endpoint being the one
    source of truth for where the node is, so a fresh node needs only its
    transport and its group; a program still calls `connect` itself to bring
    the wire up before any group exists, or to place the two wire actors on a
    pool of its choosing. The third overload that took the two wire actors
    already spawned went with the reason for it — an actor's member could not
    hand its transport to a spawn on Rust before [effect-handle]; now the
    group inherits it.
  * **`net`'s public surface is the layers a program touches** (2026-09-27):
    the transport effects and handlers, `encode`/`decode`/`protocol`,
    `NodeId`/`this_node`/`new_node`/`pool_at`/`node_of`, `connect`/
    `connected`, the readers `credits`/`pending`/`peer_protocol`, the groups
    and their fns, the route kit — plus `route_pick`, `RouteConfig` and
    `key_hash`, exported because the generated `route_any` stubs call them
    from the program's module.
    The runtime's own bindings (`add_route`, `route_frames`, `deliver_frame`),
    the control channel the group protocols travel on (`watch_control`,
    `send_control`, `control_frame`, `node_left`, `local_protocols`,
    `set_peer_protocols`), the handshake's own fns (`hello_frame`,
    `handshake`, `introduce`, `leave_group`, `share_members`) and the view mirror (`view_set`, `view_members`,
    `view_version`, `view_refresh`, `view_wait`) are **private to
    `net`** [mod-export]; each carries a comment saying what it does and where
    it sits.
  * **Replies arriving over the wire are decoded by whoever knows the
    answer's type**: an actor's generated `decode_reply` (off its parked
    continuation, when the activation runs), a waiter's decoder registered by
    the `waitfor` site, a task's by its mint — the runtime holds bytes and
    cannot name a Salvo type. Every `spawn` hands the runtime the actor's
    message decoder (`__DECODE_H`, by protocol hash) for the same reason.
  * **Encoding and decoding happen with the scheduler lock released** — an
    `Addr` or `Reply` inside a payload asks the scheduler for its identity.
    Found the first time a `total(out)` crossed: a deadlock inside
    `salvo_send_wire`, then another inside frame delivery; both runtimes now
    do every codec call in a lock-free phase.
* [addr-capability] **The bits are what make a wire addr a capability.**
  Minted unguessably at every spawn (OS-seeded hashing on Rust,
  `SecureRandom` on Kotlin), carried in every crossing identity, and checked
  on delivery — a frame whose bits do not match the actor at that index is
  dropped, and an identity that imports with wrong bits answers a dead entry,
  so sends to it are the silent no-op every send to the dead is. This is what
  keeps "who holds which face decides what they may do" true across a
  machine: the second face's addr has bits the first's does not.
* [remote-backpressure] **A remote mailbox's `capacity` stays true through
  credits.** A proxy is born with none and sends an OPEN; the host answers a
  GRANT of the room left (`bound − queued − already granted`, at least one);
  each send spends a credit and **blocks at zero**, exactly as a local send
  blocks on a full mailbox; each dequeue of a remote sender's message grants
  one back. So `bound` bounds the queue with remote senders too (at most one
  over per starved sender, which the local send's own block absorbs).
  `credits(addr)` reads a proxy's balance — what a pick will read as a
  remote member's load (step ⑤) — and answers `None` for a local actor.
  Delivery is **at most once, in order per (sender, receiver) pair**: the
  transport's guarantee, and nothing more.
* [node-exit] **A node that leaves takes its proxies with it**: the routing
  service forgets the node's route and kills each proxy of an actor there
  with the reason that the node left, so a send to one is the silent no-op
  and each watch of one is answered with an `Exit`, as for a local death
  [actor-watch].
* [node-group] **Membership of nodes is one actor effect whose handlers are
  the mechanisms** (step ④, user decision 2026-09-26, after a separate
  `NodeDiscovery` effect collapsed twice: its `authoritative()` flag was a
  plain handler steering a std actor's algorithm, which meant the algorithm
  *was* the mechanism). `actor effect NodeGroup { members(out), subscribe(w),
  leave() }`, `NodeGroupWatcher { joined(n), left(n, why) }`, `Node { id: NodeId,
  at: NodeEndpoint }` — what a mechanism knows *after* contact, where a
  `NodeEndpoint` is what it knows before. **A mechanism starts itself** in its
  `init` block [handler-init] (2026-09-27; until then `node_group(spawn H(…))`
  sent it a `join(events)` carrying the face a spawn answered, because a
  handler could not name its own address): `let nodes = spawn
  StaticNodeGroup("demo", all) on p` is the whole of it. It learns the node's
  own endpoint from `Transport.local_endpoint()`, so that is not a constructor
  parameter, and it **connects the node** as it starts when nothing has
  [net-connect].
  * **The handshake is std's, written once in `net.sv`** (moved out of the
    runtimes 2026-10-02, the runtime record), common to every mechanism through
    `handshake(group, from, data)`: a `Hello` (group name, the sender's
    endpoint, its **protocol table** — every actor effect with a wire form
    and its hash, registered by `main`'s prologue and read back with
    `local_protocols()`) handed to the transport directly as a control
    frame for whichever node receives it, since no route exists yet; an
    `Ack` back with the same; the group name compared on both sides so two
    deployments on one network refuse each other by name; the route and the
    peer's table recorded (`add_route`, `set_peer_protocols`); a `Leaving`
    on departure, whose `node_left` also kills every proxy of an actor on
    that node (sends become the silent no-op, watches fire with `node
    left`). The runtime only carries **control frames**: a channel and a
    payload — here the canonical encoding of `Handshake = Hello | Ack |
    Leaving | Intro` [wire-format] — handed to the actor listening on the
    channel at that node as its **private** `control(from, data)` member
    [actor-private-send], registered from its `init` with
    `watch_control("", self@NodeGroup)`. (Until 2026-10-02 the runtime ran
    the handshake and delivered `hello`/`gone`/`introduced` messages it
    built; until 2026-09-27 those were a `PeerEvents` face.)
    `peer_protocol(node, name)` answers a peer's hash for a protocol.
  * **A subscriber hears every node already known** (2026-10-02): a
    mechanism's `subscribe` first sends `joined` for each known node, then
    reports arrivals and departures — which is how a replica opened after
    the handshake learns of its peers.
  * **Introductions are control messages, not a protocol**:
    `introduce(node, peers)` sends an `Intro`. Gossip
    was first written with a `NodeLink` actor effect the group both served
    and sent to, and the deadlock graph warned of the `NodeLink → NodeLink`
    send cycle on every program importing `net` [actor-deadlock-cycle] — a
    correct warning about an inherent cycle, so the cycle was removed: the
    events are frames in, private messages out, and the group never sends
    the protocol it serves.
  * **std ships two mechanisms**: `StaticNodeGroup(name, all)` — every
    endpoint known up front, a HELLO to each — and `GossipNodeGroup(name,
    seeds)` — a HELLO to the seeds, and on every `hello` the newcomer is
    introduced to everyone known and everyone known to the newcomer, so any
    connected seed set converges on a full mesh (three nodes each seeded with
    the first see two peers each). Both `[Transport, spawn]`, both bound per
    node. **Recorded follow-ups**: a partition policy for gossip (unannounced
    departure is only a LEAVE today; a failed `deliver` should become
    `left(n, "unreachable")`), and `HeartbeatNodeGroup` over a `Ddb` platform
    effect as the interop example.
* [effect-generic-decl] **A generic that is only ever an effect is erased.**
  A type parameter that occurs solely inside `Addr<…>` (or as the argument
  of another erased declaration, or nowhere at all on a struct whose every
  instantiation names an effect) carries no representation: `Addr<E>` lowers
  to the same handle for every `E`. The checker sees `Reg<E>`, `actor_group<E>`,
  `ActorGroup<E>` as ordinary generics — `Addr<ActorGroup<Ping>>` is checked
  as such — and both emitters emit them **monomorphic**: `salvo_core::erase`
  computes the erased set as a fixpoint over the program (self-reference
  allowed, so `actor effect ActorGroup<E> { peer(other: Addr<ActorGroup<E>>) }`
  erases), then emits a copy with those generic lists cleared and drops the
  type arguments at every use — no turbofish, no `<T>` on the Kotlin fn.
  Erasure is per declaration, not per name: `eq<E>(a: Addr<E>, b: Addr<E>)`
  erases beside `eq(a: Box<T>, b: Box<T>)`, which keeps its `T`. A generic
  that *also* occurs outside `Addr` stays a generic.
  * Because the emitters match declarations by address between the resolver's
    scopes and the symbol table, both are rebuilt over the erased copy; the
    checker's span-keyed side tables from the original still apply, since
    erasure changes no span.
  * **One scope, one instance of an erased effect** (2026-09-27). `RouteSelector<A>`
    and `RouteSelector<B>` are one type in the output, so a `use` binding the second
    while the first is visible is refused, naming the remedy — a nested
    scope, which in Salvo is a function (`examples/cluster/`'s `two_ids` and
    `shop`). Shadowing the *same* instance stays legal [effect-intercept].
    Lifting the rule means a phantom parameter on the erased trait and a
    marker type per effect on the Rust side; recorded in ROADMAP.md.
* [actor-group] **`ActorGroup<E>` is the routable set of `Addr<E>` a program
  spreads over its nodes**, a std actor effect: `join(member)`, `leave(member)`,
  `members(reply)`, `subscribe(who: Addr<ActorGroupWatcher<E>>)`. The replica
  listens on its group's name with its private `control(from, data)` member,
  registered in its `init` (`watch_control(name, self@ActorGroup<E>)`)
  [handler-init]; until 2026-10-02 the runtime delivered private
  `peer(node)`/`merged(from, found)` messages instead, and until 2026-09-27
  those were members of the effect and a `start(me)` handed the replica its
  address.
  `actor_group<E>(nodes) -> Addr<ActorGroup<E>>` (and `actor_group<E>(name,
  nodes)` for several groups of one protocol; renamed from `attach` 2026-09-27)
  spawns the std `ActorGrouping<E>` handler on the current node and
  **listens by name**, so a replica opened under the same name on another
  node finds it: on every `joined(node)` its node group reports, the replica
  shares `(its hash of the protocol, its members)` with that node's replica
  of the same name, as a control message rather than an actor send so the
  group never sends the protocol it serves [actor-deadlock-cycle]; a replica
  hearing from a new peer whose hash matches its own shares back, so the two
  converge whichever opened first, and one whose hash differs is invisible
  [protocol-hash]. Replicas merge
  their member sets, admit a remote member once, and withdraw every member
  hosted on a node that leaves. `join<E>(group, member)` is the ordinary
  send; `members` answers the union as seen locally.
  * **`actor_group` is an ordinary fn over an implicit** (user decision
    2026-09-28): `fn actor_group<E>(nodes: Addr<NodeGroup>, ?protocol: () ->
    Protocol<E>) [spawn]`, its body `spawn ActorGrouping<E>(name,
    protocol())` and the node-group subscription. The body cannot know `E`'s
    name or hash — an effect-only generic is erased [effect-generic-decl] —
    so the value carries them: at `actor_group<Ping>(nodes)` the checker
    resolves `?protocol` to std's `intrinsic fn protocol<E>() -> Protocol<E>`
    [implicit-resolve] and the emitters pass it as an adapter closure
    yielding the literal [implicit-intrinsic]; a generic caller declaring its
    own `?protocol` forwards it [implicit-forward]. Until then the two
    overloads were intrinsics lowered per call to `open_group(protocol<E>(),
    …)` / `open_named_group(name, protocol<E>(), …)`; those two fns are
    folded in, and the unused private `protocol_name`/`protocol_hash`
    intrinsics deleted. **`protocol<E>()` is now the only intrinsic whose
    lowering reads its type argument** (`decode<T>` reads one too, but to
    pick a codec for a *type*, which is the pattern's other candidate — see
    ROADMAP.md); a program never needs to write it, and a builder of groups
    is a plain fn taking `?protocol` or a `Protocol<E>`.
  * **`spawn H(…) on p in group`** (user decision 2026-09-27) joins the spawned
    actor to `group` — the spawn stays local, on its pool, and the clause adds
    the one `join` send. `in`, not `on`: a group is a *place the actor can be
    found*, not a placement, and a spawn never runs anywhere but here (no
    remote spawn). `group` must be an `Addr<ActorGroup<E>>` for one of the
    handler's faces; with several faces the addr of that face joins and the
    spawn still answers its tuple. Refused, naming the faces, when the group
    is of none of them, and when the clause is not a group handle at all.
    Emission: the spawn, then `__Msg_ActorGroup::Join(addr)` over the wire
    send std's `join` makes, the spawn's value unchanged (`spawn_joins`
    records which tuple element).
  * **Two levels, one program**: a node group (`spawn StaticNodeGroup(…)`) is
    about *machines* — a program has one, started once per node, and it
    connects the node if nothing has [net-connect]; `actor_group<E>(nodes)` is about *the actors of one
    protocol across them* — a program opens one per protocol it routes to or
    lists the members of, on every node that hosts or reaches them.
  * **The crossing site for `noremote`** [noremote] is wherever
    `protocol<E>` is resolved for a concrete `E`: written directly as
    `protocol<Ping>()`, or filling an implicit `?protocol: () -> Protocol<E>`
    at a call such as `actor_group<Ping>(nodes)` (or a program's own generic
    fn that declares the implicit). It refuses when any `send fn` of `E`
    carries a payload with no wire form — "a group of `E` cannot span nodes",
    naming the call the program wrote — so an `Addr<E>` that could not be
    routed is never published. Inside a generic body the implicit is
    forwarded, not resolved, so the refusal lands at the outermost concrete
    call, which is where the program named the protocol.
  * `pending(addr) -> Int` answers the mailbox depth of a local actor and the
    in-flight (granted, unacknowledged) count on a proxy
    [remote-backpressure] — what a load-aware picker reads (step ⑦).
  * `eq(a: Addr<E>, b: Addr<E>)` is identity on both backends, so members can
    be compared and deduplicated; `node_of(addr) -> NodeId` answers the host.
  * **Naming**: node groups and actor groups stay visibly distinct —
    `NodeGroup`/`Node`/`NodeGroupWatcher`/`NodeEndpoint` for the machines,
    `ActorGroup<E>`/`ActorGroupWatcher<E>` (and `RouteView<E>`/`RouteMember<E>`
    for routes) for the actors; `Addr<E>` keeps its name.
* [route-stub] **`use route_any(group)` binds `any E` to whichever member a
  selector chooses, per send** (names user decision 2026-10-03: the route,
  `route_any`, `RouteSelector`). `group` is an `Addr<ActorGroup<E>>` for a
  non-generic actor effect `E`; the binding is a handler `of any E` the
  compiler writes — `__Route_E(group, config: RouteConfig)
  [RouteSelector<E>] of any E`, one per protocol a module routes, with the
  view version its selector last saw as state; each member is
  `route_pick(group, config, seen[, key_hash(k)])`, then a forward to the
  answer — appended to the module before resolution (`salvo_core::route`)
  and constructed where the program wrote `route_any`. `use
  route_any(group)` is expanded to `use route_any(group,
  default_route_config())`.
  It is generated **syntactically**: a module containing `use route_any(…)`
  gets a stub for every actor effect it names in an `actor_group<X>(…)` or
  `protocol<X>()` call or an `ActorGroup<X>` type; a `use route_any(g)`
  whose protocol the module never spelled is refused, naming that fix. The
  stub imports what it needs — the kit from `net`, the protocol's own types
  from their modules.
  * **`RouteSelector<E>`** is the policy, in two phases (user decision
    2026-10-03, option B): `changed(view: RouteView<E>)` runs when the view
    has moved since the selector last saw it, and the selector keeps what it
    derives in its own state; `select(view, key: Long?) -> Addr<E>?` runs
    per send, `key` being the hash of the `Key` argument. The view carries
    nothing about selection: `RouteView<E> { members: List<RouteMember<E>>
    }`, `RouteMember<E> { addr, local }`, members in a stable order (by
    node, then actor id — the same on every node and both backends). Load
    is read per send with `pending(addr)`.
  * **Waiting** (user decision 2026-10-03): `None` makes the send wait,
    `RouteConfig.first_wait` (1 ms) doubling up to `max_wait` (5 s), waking
    at once whenever the view's version moves; it waits as long as it
    takes, with no give-up. An empty group and a group without a leader
    wait rather than fail.
  * **The view is the runtime's mirror of the local replica**, kept by the
    routing service with a **version**: the `ActorGrouping<E>` replica
    writes its member set after every change (`view_set`), and
    `ActorGroup.refresh()` moves the version alone — what an election calls
    when its answer changes, so a leader change is a view change too. The
    stub reads it on the sender's thread (`view_members`, `view_version`,
    `view_wait`) — behind the replica by one message, never in the send
    path, and no hop. A group with no replica on this node has an empty
    view, so its stub waits: route a group from the node that opened it.
  * **`Key`** (`export provenance qualifier Key<T> of T`, std `net`) marks
    the parameter whose value decides the member: `send fn reserve(sku: Key
    Str, …)`. Read syntactically by the stub generator, **erased at
    lowering**, so the parameter's type is the plain one and a caller iterators
    a plain value; at most one per member, refused at the declaration
    otherwise. The stub hashes the argument's wire encoding (FNV-1a, both
    backends) into the `key` `select` receives.
  * **std ships three selectors**: `LeastLoaded<E>(prefer_local)` — the
    lightest queue, local members first when asked; `Sharded<E>()` — `key
    mod n` over the ordered view (a keyless send goes to the first member;
    consistent hashing is a recorded follow-up); `Elected<E>() [Leader]` —
    the member on the node `Leader.leader()` names, asked in `changed` and
    kept, waiting while there is none. `Leader { fn leader() -> NodeId? }`
    is a plain effect a Salvo election or a platform handler serves; std
    ships `StaticLeader(node)`.
  * A selector's instance `RouteSelector<E>` is a generic effect instance whose only
    argument is an effect: [effect-generic-decl] erases it to the
    monomorphic `RouteSelector`, which is why it may be captured as an owned handle
    where a `Random<Int>` dependency still could not (`effect_only_args`).
  * The replica now wears `NodeGroupWatcher` as a second face: a node's
    departure withdraws every member it hosted, telling the subscribers.
* Two emitter facts the module surfaced, both fixed with it: **`send(reply,
  None)` on Rust** boxed an `Option<_>` rustc could not infer, so the box is
  now typed from the token's payload (`Box::<Option<usize>>::new(None)`)
  [rs-actor]; and **`copy` of an `Addr`/`Pool` on Kotlin** was refused — both
  are scheduler indices, so a copy is the reference itself [kt-copy].

## Deductions

* [deduce-syntax] The **deduction clause** — `=> entry, entry, …` after the
  return type (or after the effect list when there is no return type), on
  the same line or the next; the body's `{` follows its last entry. **Written
  on the next line it takes the *declaration's* indentation, not the body's**
  (user preference 2026-09-22): the clause is part of the signature, and
  indenting it into the body made it read as the first statement. Every source
  in the repository is written that way — states
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
    | `=> list: +A B` | exhaustive, with `A` **re-established by this function** [deduce-reapply] |
    | `=> .f: proj(a)` | the result's field `f` projects `a` [proj-infer] |
    | `=> v.f: proj(a)` | the parameter `v`'s field is re-pointed to project `a` [proj-infer] |
    | `=>[f] entry, …` | a group: entries about the fn-typed parameter `f`, whose own parameters are named in its type [fn-contract] |

  * **The opaque lends are not a clause entry** (in the clause until
    2026-09-24, prefixed on the return type until 2026-09-25; both older
    spellings stopped parsing): a result that is owned but *holds* borrows
    somewhere inside says so **after the return type** —
    `-> T holds proj(a, b)` [proj-infer]. Read as a pair with the wholesale
    qualifier, one word each: `proj(x) T` says the value **is** a borrow of
    `x`, `T holds proj(x)` says it *holds* one — and the type reads first,
    which is what the prefix form got wrong (it looked like a qualifier
    applied to a parenthesized type). No parentheses of its own: `proj(…)`
    carries them, which is also what keeps the form unambiguous inside a fn
    type's parameter list. **`holds` is reserved**, not contextual, because a
    type is a chain of space-separated qualifiers and a bare word after one
    reads as another qualifier. The same form sits on a fn *type*'s own
    return (`?iter: (c: C) -> Mut It holds proj(c)`), which is where the
    remote `=>[f] proj(c)` group entry went. The parser synthesizes the
    internal entry (`DeductionTarget::Opaque`), so validation,
    `declared_lends`, the emitters and the hover's lent flags are unchanged
    by either respelling.
  * [proj-infer-fn-type] **On a fn type whose return is opaque, the lend is
    inferred** (user decision 2026-09-26): a return that is a bare type
    parameter — which every iterator-minting slot has (`?iter: (c: C) -> Mut It`) —
    may instantiate to a type holding a borrow of what the slot was given, and
    only a lend makes that borrow nameable. So every **named, kept** position
    of such a fn type lends, conservatively, which is what this rule's fallback
    already says for a bodiless declaration — and a fn type *is* a bodiless
    declaration.
    * **Why it is free**, probed before deciding: the lend is *strictly more
      permissive* than its absence. A candidate whose result holds no borrow
      simply does not use the lifetime, so `core.set`'s snapshotting `iter`
      fills a lent slot exactly as `core.list`'s borrowing one does. No program
      that compiled without the lend stops compiling with it.
    * **One shared lifetime, not one per parameter.** The opaque return's hidden
      lifetime has to *be* one of the parameters', and separate lifetimes leave
      the target compiler no way to know which — so all the tied positions share
      `'c` [rs-proj-lends].
    * **Two things scope it**, both from the same principle — a lend names a
      source, and the source must be a parameter here: a position with **no
      name** infers nothing (std's `?copy: (T) -> T`), and a **concrete** return
      infers nothing, because a concrete type says for itself whether it borrows.
      Together these are why no existing signature changed meaning.
    * **Writing it anyway warns** (user decision 2026-09-26): on an
      opaque-returning slot the clause says what the signature already infers,
      and an annotation that changes nothing reads as evidence that something
      needs saying — the same reason a no-op `@place` selector warns
      [fn-overload-at]. A warning rather than an error, since the program is
      correct and the clause is merely spare; **one diagnostic per clause**,
      naming every source it spends (`holds proj(c, k)`), because that is one
      thing the author wrote and one thing to remove. Silent where the return is
      concrete, where the clause is load-bearing.
    * Before it, the most ordinary generic-iteration signature in the language
      type-checked, ran on Kotlin, and failed at rustc with "lifetime may not
      live long enough" in code the author never wrote — an accept/reject
      divergence [backend-parity] whose fix the author had no way to guess.

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
    keeps its default (kept) and its *parameter* entries are written only
    through `=>[f]`, never inline (`f: (v: T) -> [v] U` is a parse error:
    ambiguous with the next parameter) — its *lends* are the one inline
    statement, after its own return type (`-> Mut It holds proj(c)`), which
    the next parameter cannot be confused with because the sources sit inside
    `proj(…)`.
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
    parameter (a deduction preserves, drops or **re-establishes** one of the
    parameter's own qualifiers — `+Qual` [deduce-reapply] — and never adds
    another); an entry is either
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
* [deduce-reapply] **`+Q` in a function's own clause says the function
  *establishes* `Q`**, rather than that the body preserved it (user decision
  2026-09-22, which is ROADMAP's D2 answered). It is what lets a **mutator keep a
  user qualifier**: a mutating callee's exhaustive entry must strip the claim
  [deduce-syntax], and before this the only shape that could keep one was
  consume-and-return through a constructor fn — so `Mut`-parameter APIs, the
  language's own idiom for mutators, could never preserve a qualifier while
  std's `intrinsic fn`s could (they have no body to validate).
  * **Written `+Q` inside an exhaustive list**, beside the claims that merely
    survived: `=> heap: +Heap<T>(?cmp) Mut`. The caller sees the union — an
    exhaustive list is still *these and nothing else* — and the distinction is
    about who vouches for each: a plain name is checked against the body, a `+`
    name is trusted. Two spellings because they are two different statements, and
    the reader can tell which is which (the user's refinement of the proposal).
  * **Only in the file that declares `Q`** [qual-ctor-same-file], which is the
    same party already trusted to mint the claim with `-> +Q T` [qual-ctor-fn]
    and to speak for someone else's call with a `refn` [qual-refn]. Elsewhere it
    is an error naming both remedies.
  * **It establishes, whether or not the parameter already holds the claim**
    (extended by user decision 2026-09-22, the heap demo's `heapify`): re-applying
    what a mutation stripped and *minting* a claim on a value that arrived without
    one are the same statement — "after this call, this is a `Q`" — and the same
    party is trusted for both. `heapify(list: Mut List<T>, ?Ordered<T>)
    => list: +Heap<T>(?cmp) Mut` takes an ordinary list and hands back a heap,
    which the plain spelling could never say: it may only preserve what the
    parameter declares.
  * **A minted claim answers to a constructor's rules** [qual-ctor-fn], since
    that is what it is: the qualifier must **apply** to the parameter's type
    [qual-of], and a qualifier that *holds a function* must be given one —
    `+Heap` with no arguments establishes a claim whose identity nothing named,
    which every operation reading the claim would then fail to bind [cmp-binder],
    so it is refused naming the slots.
  * **Re-establishing keeps the parameter's arguments.** When the parameter
    already holds `Q`, written arguments must **agree** with it (`+Heap<T>(?cmp)`
    names the claim precisely; a *different* identity is refused, because a value
    of a different type belongs in the return type), and omitted arguments mean
    the parameter's own [cmp-binder].
  * **The identity is the call's, not the callee's.** A minted claim's arguments
    are substituted at each call site from the resolved type arguments and
    implicits, so `heapify(xs)` and `heapify(xs, cmp = by_name)` produce
    `Heap<Person>(cmp@Person)` and `Heap<Person>(by_name)` — two types, which is
    the whole point of the identity living in the type [cmp-carry]. A `?cmp`
    written in a deduction clause is therefore a **binder occurrence** like one in
    a parameter or return type is.
  * **A compiler qualifier cannot be re-established** (`+Mut`, `+proj`): those
    are representation choices rather than claims about the value, nothing could
    establish one, and `Mut` is not even erased.
  * **Not in a `=>[f]` group** [fn-contract]: that states what a *callback* does,
    and neither side of it could earn the trust — the declarer of the callback's
    own fn is who may claim it, with a `refn`.
  * **No emitter work**, and none possible: qualifiers erase [qual-erasure], so
    the whole feature is a checker rule. `demo/heap.sv` is what it was decided
    for — a heap in user space whose `heap_push`/`heap_pop` take `Mut` parameters
    — and that program now compiles and runs on both backends.
  * Rejected: **full D2** (an establishment *proof* — verify every path called
    something that establishes `Q`) needs a per-qualifier establishment relation
    that does not exist, for a rule the same file could simply assert; and
    **staying with consume-and-return**, which was the status quo the decision
    overturned.
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
    `proj(p)` is a borrow, not a move [readonly-return]), `break`-ed,
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
  (`proj(p)` [readonly-return]), in which case the result
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
  * **A handler member's clause is its contract too** (2026-09-30): the
    member's written list decides which of its parameters it owns, exactly
    as a fn's does, so `let {body} = input` in a member consuming `input`
    is move-mode. Until then a member had no contract in scope and every
    derived binding in it stayed read-only.
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
    body is legal; the loop re-check re-declares it per iterator). When a
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
  * a read of a **disjoint** projection iterators — `eat(p.tags)` then
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
    expected (a pure lambda iterators to a `[Logger]` position and simply
    ignores what it is given); never the reverse, which would reach a call
    site that cannot supply it. Same direction as [fn-contract] and
    [once-fn].
  * **Inference**: an un-annotated lambda's effect set is inferred from its
    body (inference from a *visible* body is what [decl-explicit] permits),
    so it cannot silently fit a pure position. Where a fn type is written,
    its list is authoritative — including for emission, since a pure lambda
    in an effectful position must still *take* the parameters the caller
    iterators.
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
    ([rs-handle]; [kt-effect-params]).
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
    type. Driving a named iterator advances it in place
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
* [readonly-return] `-> proj(p) T` marks a **projected return** (L7c,
  2026-09-02; `proj(p)` as a qualifier on the type, user decision
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
* [proj-anywhere] `proj(a, b)` is a qualifier writable wherever a
  type is (user decisions 2026-09-11): the whole result, a union arm
  (`Emitted (proj(p) T) | Finished`), a nullable, a tuple element, a
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
* [proj-readonly] A `proj` value **without `Mut` is read-only** (user
  decision 2026-09-11; narrowed 2026-09-24 by [proj-mut], which is P-3 of
  the group-borrowing ladder): a plain projection never satisfies a `Mut`
  position; passing it where `Mut` is required, or mutating it, reports the
  projection and the `copy` remedy (and, since [proj-mut], the
  `List<Mut T>` route). `copy` is the way out and yields an owned `Mut X`.
  Moving a wholesale projection is refused the same way — `Mut`-carrying or
  not — except a Copy scalar [copy-scalar-free].
  * A **parameter** written with a top-level `proj Mut` is an error at the
    *declaration* (user decision 2026-09-12): the `proj` promises to accept
    borrowed values, but the `Mut` makes the position one no projection can
    satisfy, and the body could never use the permission either. The
    diagnostic names both remedies (drop the `Mut` — a `proj` position
    accepts `proj Mut` arguments — or drop the `proj`). Nested occurrences
    (`Mut List<proj Mut Str>`) and non-parameter positions (locals, fields,
    returns) keep the type legal.
* [proj-mut] **A projection carrying `Mut` is a mutable element handle**
  (user decisions 2026-09-24 — P-3 and P-9 of COMPLETED.md's log (the group-borrowing ladder);
  the partial repeal of [proj-readonly]'s original blanket rule). Element
  mutability is the **element type's, not the container handle's**:
  container `Mut` is *structural* permission (`add`/`remove`/`set`/`swap`),
  while `List<Mut T>` elements hand out in-place-mutable handles — the
  generic `get` instantiated at `T = Mut Entity` answers
  `(proj(list) Mut Entity)?` by ordinary substitution, and that `Mut` is
  the permission. The rules:
  * `proj Mut X` **satisfies a kept `Mut X` position** — the acceptance
    that [proj-readonly] refused; a projection without `Mut` still never
    does. Consuming positions still refuse every projection.
  * **Mutating through the handle is legal** — a projection assignment,
    `++`, a `Mut` argument position — and is a mutation event on the
    handle's *roots* at the linked paths [fate-poison]: sibling derivations
    of the container fall (a computed index may-aliases every element
    [fate-field-disjoint] — unless a live `NotEq` claim proves the two
    apart [elem-distinct]), the acting handle itself survives (its storage
    did not move), and a parameter root is recorded as mutated so its
    inferred contract takes the exhaustive form [deduce-syntax].
  * **Mode is inferred per binding** (P-9, the [fate-move-mode] pattern):
    a handle some downstream use mutates is recorded in
    `Checked::handle_muts` at its bind event, for the Rust backend's
    rendering [rs-elem-mut]; handles only read keep today's borrow
    renderings, so any number coexist.
  * The **declaration-site refusals stand**: a parameter written top-level
    `proj Mut` is still an error ([proj-readonly]'s declaration half), and
    a `proj Mut` that is not element-anchored has no minting surface.
    Destruction under a live handle needs no new rule — any structural
    `Mut` use of the container poisons the handles it lent.
  * Implementation: `arg_fits_param` (the acceptance), `fate_mutation_at`
    (the mutation event, with the acting handle exempted from its own
    poison — `poison_derived_except`), `handle_muts` (the P-9 table).
  * Implementation: the projection lives in the lowered type
    ([proj-type], 2026-09-12; before that it was erased and carried only
    on links) *and* on the fate link — `FateLink.borrowed && !held` is a
    wholesale projection, `held` a borrow an owned object carries
    [proj-infer]. A variable whose every link is held may be mutated (its
    own fields are its own); one with a wholesale or alias link may not.
* [param-const] **A parameter is a constant binding** (user decision
  2026-09-25): assigning one — `n = n + 1`, `n++`, `h = Holder {…}` — is an
  error, whatever its type. The reading is that a parameter is `const`, and
  a future `const` on **struct fields** will say the same thing for scalars
  and non-scalars alike, so one rule holds everywhere rather than a
  by-type split. The remedies the diagnostic names: bind a local
  (`let next = n + 1`), or assign a **field** of the parameter
  (`h.tags = …`) to change what the caller holds. A handler's *state* field
  is not a parameter in this sense and stays assignable.
  * Closed a defect rather than added a restriction: the construct was
    accepted and then refused by the target compilers — Kotlin because
    parameters are `val`, Rust for any borrowed one — so it was a
    checker/emitter disagreement [backend-never-wrong]. Nothing in std,
    the examples or the corpora relied on it.
* [deduce-field] **Field-granular mutation entries** (user decisions
  2026-09-25; rung ⑤ v1 of the group-borrowing ladder): a clause entry may
  name a parameter's **field** and so say *where* a call mutates.
  `=> h: Mut` keeps its meaning — mutated *anywhere*, poisoning every
  derivation of `h` — while `=> h.tags: Mut` narrows the event to
  `[.tags]`, and `=> !h.tags` states a **replacement**: the field's storage
  identity is destroyed (consumption, one level down — at field level `!`
  means *replaced*, not gone, since the caller keeps the field and loses
  only identity continuity; on a linear field it is the entry that obliges
  the callee to discharge what was there).
  * **No new invalidation rule.** The event a *call* produces was always
    the whole value, because it is driven by the **parameter's declared
    type** carrying `Mut`; a field entry replaces it with its own paths,
    and [fate-field-disjoint]'s overlap test does the rest. So
    `damage(e: Mut Entity, n) => e.hp: Mut` leaves a derivation of
    `e.rings` standing, and a derivation of `e.hp` still falls: the
    narrowing is precision, not permission.
  * **A field entry stands alone**: `=> h.tags: Mut` needs no `=> h: Mut`
    beside it (that would undo the narrowing). A parameter with claims to
    account for adds an ordinary non-`Mut` entry, which states survival
    without widening the event.
  * **Written entries are checked against the body**: every mutation the
    body performs on the parameter must be covered by a declared path, or
    the promise the callers rely on is a lie — an error naming the widening
    remedy.
  * **Inferred where unwritten** (2026-09-25): a callee with no clause entry
    for the parameter gets its field set read off its body and propagated
    through the deduction fixpoint, so ordinary code has the precision
    without annotating. Conservative by construction — an unknown path, a
    replacement, or handing the whole value to a mutator publishes the whole
    value, which narrows nothing. A **written** `=> h: Mut` still means
    *anywhere*: writing it is a choice, and inference never overrides it.
  * Rust: a binding that **survives** a narrowed mutation cannot be a live
    borrow (the callee still takes the whole value `&mut`), so it renders
    as a *virtual place* — the path re-materialized per use [rs-loc].
    Sound precisely because survival means the field was untouched, so the
    re-read sees the object Kotlin's binding holds. (The first attempt at
    this rung relaxed poison for *any* field path and broke that parity;
    COMPLETED.md's log (rung ⑤) keeps the probe.)
  * **v2 (built 2026-09-25): contents versus replacement.** A **contents**
    mutation (`=> h.tags: Mut`, and any call mutating through the place)
    leaves the mutated thing's own storage in place, so a derivation that
    *is* that thing — an inline handle at exactly the event's path —
    survives, and both backends observe the same object. A **replacement**
    (`=> !h.tags`, or an assignment to the place) poisons it, as does any
    derivation reaching *through* the contents. The distinction rides a
    `crosses` bit on the link, **conservative by default and deliberately
    not inferred from the path**: `first(h.tags)` records path `[.tags]`
    exactly as `h.tags` does, so only derivations the analysis can see are
    inline — a field chain or a plain alias of a place — are marked
    non-crossing; a call's lend never is. Spared handles render as virtual
    places, which is what keeps Rust in step with Kotlin (and what the
    historical `diverged` case needed: `let t = h.tags; add(h.tags, 2)`
    now compiles and prints the same on both).
* [qual-field-place] **Qualifiers hold about struct fields** (the ⑤
  follow-on, verified 2026-09-25): `if h.tags is NonEmpty { … }` narrows the
  **field place**, the claim is consumed by overload resolution inside the
  branch (the total `first` resolves), it **survives a mutation of a
  disjoint field** — which is what [deduce-field] bought — and it falls when
  the claimed field is itself mutated. No new machinery was needed: flow
  facts were already keyed by place [flow-place] and invalidation already
  took a place, so the feature was waiting on the *precision of
  invalidation*, not on a mechanism.
  * The boundary: a parameter **type** has nowhere to state a field claim
    (`h: Mut Holder` cannot say `.tags: NonEmpty`), so such claims are
    established and consumed *within* a function. Carrying one across a call
    boundary would need type-level syntax, which nothing yet asks for.
* [canbe-entry] **`canbe` — the alias-group relation** (user decisions
  2026-09-24, GB-1(s); built as rung ④b): a deduction-clause entry saying
  two parameters **may name the same object** — `=> a canbe d`. Symmetric
  (writing both directions would be noise) and **non-transitive** (the
  relation is a graph, not an equivalence). Exempt from the
  one-entry-per-parameter rule, on `preserve`'s precedent: `=> a canbe d,
  a: Mut` is two statements about `a`. Forms, all desugaring to binary
  symmetric relations:
  * **`=> track canbe in lib.tracks`** — the *anchored* form: the parameter
    may be an element of the named container path, and two parameters
    anchored in the **same** path may therefore coincide (the
    shared-anchor rule: the container-rooted n-way case costs one entry
    per parameter, linear in n). The path is **rooted at a parameter** —
    the anchor is a value the callee has — which is also what lets the
    anchored form license handles passed *beside their own container* in
    one call [rs-loc].
  * **`|` lists on both sides**: on the right a hub (`a canbe b|c` is a↔b
    and a↔c, *not* b↔c — the sentence says exactly what the rule means);
    on the left plural-subject sugar (`a|b|c canbe in es` is the three
    anchored entries). `canbe in` takes path lists the same way.
  * **What coverage buys**: the same-call rule stands down for a covered
    pair [deduce-same-call] — two element handles of one container need no
    disjointness proof [elem-distinct] — and the Rust backend renders the
    covered positions against a **shared anchor** [rs-loc].
  * Written-only (P-4): aliasability stays visible in every signature; no
    inference claims an entry. The entry says nothing about keptness or
    qualifiers. Diagnostic vocabulary keeps the word "alias group" for the
    connected component, while the surface never needs it.
  * **Both sides name parameters**, and a name that is not one is an error
    (2026-09-25, found closing the anchored form's lowering defect): the
    plain form relates two *parameters*, so a field path on its right is
    refused with `canbe in` named as the form that means it; the anchored
    form's container is a path whose **root** is a parameter. A plural
    subject reports its right-hand side once, not once per subject.
* [elem-distinct] **Distinct awareness** (user decisions 2026-09-24 —
  step ② of COMPLETED.md's log (the group-borrowing ladder)): two mutable element handles of
  one container whose minting indices a live `NotEq` claim proves apart
  name **disjoint storage**, and the analysis knows it — the first
  refinement of [fate-field-disjoint]'s may-alias-all rule for computed
  indices. The rules:
  * **Element links carry the identity of their minting index**: a handle
    minted by `get(list, i)` — resolved to `core.list`'s `get`, either
    overload — records `i`'s ultimate fate-root id on its links.
    Recognition is **nominal** deliberately: a user fn with a derived
    return may lend *any* projection of its container, so only the `get`
    whose semantics the compiler knows may name an element discriminator.
    A nested mint keeps the identity nearest the root
    (`get(get(grid, i)!, j)` carries `i` — the discriminator of disjoint
    subtrees under the shared root); any other index shape carries none.
  * **The identity dies with the index variable's value**: reassignment
    or `++`/`--` of the index erases it from every link, eagerly at the
    event — a surviving identity always means "the element selected by the
    variable's *current* value", so the poison consult needs no staleness
    check. (Loop bodies re-check under the post-iteration state, so an
    erasure late in a body reaches uses before it.)
  * **Poison consults the claim**: mutation through a handle spares a
    sibling link iff both identities are present, different, the link
    paths are equal, and a live `NotEq` claim relates the two index
    variables (either orientation, matched by fate-root agreement
    [qual-depend]). Anything short of the full proof poisons as before
    [fate-poison]. The same index minted twice is certainly the same
    element — no claim is consulted.
  * **One call may take two proven handles** — `attack(get(es, i)!,
    get(es, j)!)`, or two bound handles — recorded in
    `Checked::distinct_pairs` for the Rust pair lowering [rs-elem-mut].
    An **unproven** pair is refused at the second argument, naming the
    remedy (before this rule the shape passed the checker and failed in
    rustc, E0499 — a checker/emitter disagreement).
  * Implementation: `FateLink.elem_idx` (the identity),
    `elem_mint_index` (the nominal mint recognition),
    `erase_elem_identities` (the eager invalidation),
    `live_distinct_pairs` + the spare in `poison_derived_except` (the
    consult), and the pair pre-pass in `resolve_named_call`.
* [proj-field] **Any struct may hold `proj` fields**, written without a
  source (`items: proj List<T>`): the struct declares *that* it projects,
  each literal decides *what* (user decision 2026-09-11; replaces the
  pass-only exemption that first landed). Such a struct is an owned object
  that *holds* borrows — a **view**: its `Mut` is real (an iterator is advanced
  in place), its non-`proj` fields are its own, and it may be moved, stored
  or passed on; what it may not do is outlive its roots. A struct holding a
  view in an owned field is a view too. Writing `proj(x)` on a field
  is an error ("names no source").
  * Rust: the struct carries one lifetime, `View<'s>`, with `&'s` fields
    and `<'s>` on owned view-typed fields; every mention elides (`'_`)
    except where a lend ties it [rs-proj-lends]. Kotlin: unchanged.
  * Assigning a `proj` field re-points the borrow; a fn that does so writes
    `=> v.items: proj(other)` [proj-infer], and Rust renders the
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
    written statements decide (`-> T holds proj(c)` after the return type,
    `=> .items: proj(c)` per field — fn types carry the return form too),
    else conservatively every kept parameter
    (exact for `iter(list)`; only ever over-links).
  * A written projection statement about the result must name every lend
    the body performs; it may name more (a generic body — `filter`'s
    `add(out, x)` with `x` an element of an opaque iterator — lends through
    opacity the analysis cannot see, and the annotation is how it says so).
    A written entry takes **precedence over the instantiation fallback**
    (2026-09-12): where a substituted return holds `proj` the written
    type does not show (`-> Mut List<T>` with `T = proj Str`), a call
    links the result to *every* kept argument unless the author's
    `-> … holds proj(it)` names the lends — then only those link, still
    flowing through a temporary in the named position to its roots.
  * The caller links the result to the lent arguments, *held*
    [proj-readonly]; returning a view rooted in a local is an error ("a
    local that dies with this call"), and a returned view may be rooted
    only in the fn's own lent parameters. A re-pointing entry
    (`v.items: proj(other)`) gives the caller's variable at `v` a
    held link to `other`'s roots from the call on (the body is trusted for
    these — see ROADMAP).
  * Reserved, not built: struct-level **link parameters**
    (`struct Pair<T, U, a, b> { first: proj(a) T, … }`) as the
    per-field explicit form, should the conservative fallback ever bite;
    `[name-casing]` already makes it parse (ROADMAP).
* [lambda-view] **A capturing lambda is a view** (user decision
  2026-09-12): the closure holds a borrow of every non-Copy capture its
  body only reads, exactly as a struct holds its `proj` fields
  [proj-field] — because a body can return projections rooted in a
  capture (`i -> get(words, i)!` hands out elements of `words`), which no
  fn type can name (`proj(…)` sources are parameters; captures are
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
* [yield-proj] An iterator that walks data declares `: Yield<self, proj T>` and
  its `next` returns `Emitted (proj(p) T) | Finished` — the element is
  a projection of the iterator, which projects the source; a generator declares
  `: Yield<self, T>` and emits owned values (user decision 2026-09-11: one
  `Yield` group, the element type argument carrying `proj`). The
  obligation and the member must agree — `proj` on one and not the other
  is an error naming the fix. Reading combinators (`?Yield<It, T>`) accept
  both. std's `ListYield`/`ArrayYield`/`StrYield` borrow (`items: proj
  List<T>`; `StrYield` emits owned `Char`); an `iter fn`'s generated struct
  borrows its parameters (`list: proj List<T>`, `proj` snapshot fields, Copy
  scalars owned) and names the parameter as the elements' source
  (`Emitted (proj(list) T)`), which the desugar redirects to the struct
  parameter. Rust: the element generic is retagged to `&T` at call sites
  whose filled `next` borrows [rs-proj-arm].
* [copy-opt-in] **A copy never happens without the program opting in**
  (user decision 2026-09-11, the principle behind phase 2b): `copy(x)` where
  a copy is wanted, a `_to` function that fills a destination the caller
  provides (with `?copy` for the element type [copy-implicit]), and nothing
  else. Everything std hands back either owns fresh data (`map`, `split`) or
  projects what it was given (`get`, `first`, `iter`, `filter`, `next`)
  [proj-anywhere]; an `iter fn`'s struct borrows its parameters [iter-fn]. A
  backend that would need a hidden clone to be correct reports instead
  [backend-never-wrong] [rs-proj-arm].
* [copy-scalar-free] Moving a derived **Copy scalar** (`Int`, `Long`,
  `Float`, `Double`, `Bool`, `Char`, `Byte` — not `Str`) is a read: the
  number taken out of a borrow is the value itself on both backends, so no
  `copy` is owed (user decision 2026-09-11). Poison still applies. The
  same exemption lets a bodiless declaration leave a scalar parameter out
  of its clause [deduce-syntax], and the hover omits scalars.
  * And a call **handing one back** records no fate link at all: `proj Int` is
    `Int` — `Ty::qualify` erases the qualifier — so there is nothing borrowed to
    keep a link to, and the result travels like any owned scalar (returned,
    stored, sent). Without this, `return get(numbers, i)` on a `List<Double>`
    was refused as a view of `numbers` and asked for a `copy` that neither
    backend would emit. A non-scalar element read is unaffected.
* [core-layers] **Core states what Salvo means; a backend spells it**
  (user decision 2026-10-06). `salvo-core` holds Salvo facts: ownership
  (how a callee holds a parameter, what a read does to a value), evaluation
  order, what `copy` duplicates, which calls resolve to which declarations.
  `salvo-backend` holds opt-in helpers a backend may call (the shared driver,
  walkers, and later the rewrites that make a Salvo fact concrete in a target
  that needs it). Each backend chooses what to spell and how: that a scalar
  is copied for free, that a fn value is a `&mut impl FnMut`, that a read must
  be hoisted ahead of a mutable borrow (rustc's E0502), that a block in
  expression position needs `run {}`. None of those is a Salvo fact, so none
  belongs in core.
* [param-mode] **How a callee holds a parameter** (`salvo_core::param_mode`,
  built 2026-10-06): *moved in* (consumed: `=> !p`, or inferred so), *lent*
  (kept, read by the callee), or *lent mutably* (kept, and the declared type
  carries `Mut`, or its elements do: `List<Mut T>` lends mutable handles
  [proj-mut]). A fn-typed parameter is lent, or moved in when the fn stores
  its callbacks (a fn-typed parameter and a struct with a fn-typed field as
  the result); a variadic tail and a `once` fn group are moved in. The inputs
  differ by family: a top-level fn reads the checker's deductions; an effect
  member (and the handler members implementing it) reads the clause written
  on it; everything else is kept. Element `Mut` counts the same in all three
  (unified 2026-10-06). A position in a fn *type* follows the same rule from
  the type's own clause, which is how a lambda binds its parameters. What a
  target makes of the mode is its own ([core-layers]): Rust passes a scalar by
  value whatever the mode, spells a lent fn value `&mut impl FnMut`
  ([rs-borrows]).
* [mut-lends] **Which fns hand back a result a `Mut` position reads, and
  which cover `canbe` parameters** (`salvo_core::mut_lends`, built
  2026-10-06): the fns whose lent result serves a `Mut` position (the
  checker's `mut_lend_calls` resolved through `call_fn`, followed along return
  paths), the call sites that read one, and, per fn, the parameters its
  `canbe` clause covers and the anchor they share. A target that renders a
  lent mutable result as a locator ([rs-loc]) emits a twin of each such fn; a
  garbage-collected target ignores both.
* [copy-plan] **What `copy` duplicates is core's call** (built 2026-10-06,
  `salvo_core::copyplan`): a tree per type — share the value when no Salvo
  operation can mutate any part of it (scalars, `Str`, non-`Mut` structs of
  such, an `Addr`, a `Pool`, fn values); otherwise a `Mut Str` gets a new
  buffer, a value platform type with a `Mut` kind its host's `copy`, a `List`
  or `Deque` a new container whose elements copy by their own plan, a struct
  a copy with each still-mutable field replaced by its own copy, an array of
  immutable elements a new array. A struct reached again inside its own copy
  has no plan and is refused. A backend whose copy is always deep (Rust's
  `clone`) ignores the plan; one whose values share structure renders it
  ([kt-copy]).
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
  * **The answer may be linear** (2026-09-29, §4b step 3d): `Reply<T canbe
    linear>` and `send<T canbe linear>`, so a stream can travel back inside an
    answer (`Reply<Received>`, whose `Ok` arm holds a `Packet` holding an
    `InStream`). The obligation moves with the value: `send` consumes it, and
    the continuation — a `waitfor`'s value or a `send fn`'s last parameter —
    owes it on arrival, checked like any linear value (a continuation that
    drops it is the ordinary leak diagnostic).
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
  * **The casualty that remains** is the composed linear iterator (a wrapper iterator
    over `open_lines(path("a"))` stores its source in an unopted field). The
    fallible-open shape `Ok InStream | Err Checked<FsError>` was the other one and
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
      check *is* the union narrow [linear-union-arm].
    * [proj-linear] **reading in place**: `get`, `at`, `first` (and a deque's
      `get`/`first`/`last`) accept linear elements and answer a projection
      (user decision 2026-10-02; closed until then). A projection may be read
      and, through `Mut`, mutated, but it carries no obligation and cannot
      take one on: giving it to a consuming parameter is the existing "a
      borrowed value cannot be given away" error, and `copy` refuses a linear
      value. So no obligation is discharged twice through one, and the
      element still owes inside its container. What the runtime's actor
      table needs (runtime steps).
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
  makes std's `drop` refuse a linear iterator while filling the same
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
    `remove_at`, `drain`; `remove`, `replace`, `drain` on a map) and, since
    2026-10-02, the borrowing reads (`get`, `at`, `first` [proj-linear]),
    while `put` stays out (it drops what it overwrites) and `copy` refuses with a dedicated
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
* [struct-opaque] **`opaque struct S { … }`** (user decisions 2026-10-05,
  ROADMAP §0j step 6c; built 2026-10-05): an ordinary struct in its
  declaring module and that module's `*.test.sv` annex (module `m.test`
  [test-file]); everywhere else the type is usable and its fields are not —
  no literal, field read or write (`s.f`, an assignment target included), spread
  or destructuring. A contextual modifier, like `noremote`, combinable with
  `linear`, `noremote` and `export`.
  * The diagnostic names the type as opaque outside its module and lists its
    functions (exported or attached fns whose first parameter is it).
  * Settled with the decision: `by auto` instances, `copy` and the codecs are
    generated in the module and usable everywhere; another module's
    qualifier cannot read the fields (its `qualifies` is other-module code);
    the host class keeps its fields under the host contract, so the backends
    change nothing.
  * **A comptime fn instantiated outside the module sees kind `opaque`**
    (`Basic`): `[when field.type]` takes the `Opaque` arm, so `by auto` and
    the codecs call the type's own `eq`/`encode` instead of walking fields,
    and `fn same(a: Bag, b: Bag) by auto` there is refused naming the
    module to stamp it in [fn-by].
  * Chosen over private fields and over a sealed (read-only) struct: opacity
    is what lets a module change a representation later.
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
  * **An `iter fn`'s generated iterator struct type inherits its visibility** [iter-fn]: a
    `for` over the iterator needs the *type* in scope, so exporting the function
    while hiding its struct would make the iterator undrivable from another
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
    internal helpers (`earliest_due`, `mem_*`, `fs_resolve`,
    `MemRead`/`MemWrite`) are now genuinely unreachable instead of merely
    undocumented.
* [mod-import] `import path.Name` / `import path.Name as Alias`; aliasing
  resolves ambiguity. Unresolved/ambiguous imports are errors.
  * Import prefixes match module paths exactly or as a leading path
    (`import core.Str` finds `core.string`).
* [mod-suffix] **A module *selector* names a module by any unambiguous suffix
  of its path** (user decision 2026-09-29), and the rule is the same for every
  selector: `size@list` for `size@core.list` [fn-overload-at], `by auto` for
  `core.auto` [obligation-by]. A suffix that fits two modules is refused naming
  both — "`@list` is ambiguous: it is a suffix of `core.list` and `shop.list`"
  — and the remedy is more of the path. The full path always works. **An
  `import` is not a selector and stays fully qualified** [mod-import]
  [mod-import-module]: it says where a name comes from, so it writes the whole
  path. Chosen over requiring the full path at selectors (`by core.auto`,
  verbose at the most common clause) and over special-casing `by`, so that a
  later `std.` prefix on std's modules changes no selector that did not spell
  it.
  * A shadow copy of std (the repository root opened as a workspace loads
    `std/` again as `std.core.auto` beside the embedded `core.auto`, ROADMAP
    §4) is not a second candidate for `by`: same content, module ending in the
    std module's path, the copy is skipped [std-shadow].
  * `ModulePath::matches_suffix` / `text_matches_suffix` in `salvo-core` are
    the one predicate; the selector sites in the checker and the comptime
    expansion read it.
* [mod-std-internal] **`runtime` is std's own module** (2026-10-02; user
  decisions of 2026-10-01, runtime E8): `std/runtime.sv`, and any
  module under `std/runtime/`, may be imported by std's files and by no
  others — whole or by name, whatever it exports. The scheduler every
  program runs on is written there (the runtime record), and its private
  declarations are where the language allows the exceptions a runtime
  needs (user guidance 2026-10-01). A program importing it is refused at the
  import, naming the rule. The path is the marker (`STD_INTERNAL` in
  `resolve.rs`): no syntax, and a program cannot declare a module of that
  path either, since std's own would collide with it.
* [runtime-layers] **The runtime is a core and its services** (user decision
  2026-10-02): `std/runtime.sv` (module `runtime`) implements the actor forms
  and may not use them; the modules under `std/runtime/` (`runtime.timers`)
  are services built on it, which may. The core importing a service is
  refused at the import, which keeps the dependency one-way — the scheduler
  never depends on something that depends on the scheduler. The core exports
  what services need (`Parker` and its fns, `now_nanos`); module-level
  `use` [mod-use] is allowed in both layers.
* [runtime-handles] **An addr and a pool are indices into the runtime's
  tables, and only std converts** (2026-10-05): `runtime` declares
  `addr_index<E>(Addr<E>) -> Int`, `addr_of<E>(Int) -> Addr<E>`,
  `pool_index(Pool) -> Int` and `pool_of(Int) -> Pool` as intrinsics (an `as`
  cast on Rust, nothing on Kotlin), and `reply_token<T>(Reply<T>) -> Token?`
  (2026-10-06), the core's token taken out of a reply token (`None` for one
  minted on another node), which is how `core.actor`'s `watch` and `on_idle`
  are Salvo. A program cannot import `runtime`
  [mod-export], so it cannot forge a handle; std's `core.actor` and `net` are
  Salvo over `runtime.routing` through them. `Addr`, `Reply` and `Pool` stay
  `intrinsic type`s: the backends generate the actor machinery and name them
  throughout, so a platform type would buy nothing (user decision 2026-10-05,
  reversing the day before's option (b)).
* [runtime-kept-fn] **The runtime's host may keep a fn value** (2026-10-02,
  runtime E2): a platform fn of the runtime's modules whose clause
  consumes a fn-typed parameter (`=> !body`) takes it owned, to keep or run
  on another thread — `start_thread(body)` (a daemon thread) and
  `guarded(body)` (the fault boundary, answering `None` or the fault's
  message). Everywhere else a fn value is lent for the call
  [platform-fn-value]. Rust takes `Box<dyn FnOnce(…) + Send + 'static>` and
  the argument becomes a `move` closure, so a capture that is not `Send`, or
  is used after the call, is rustc's refusal — the runtime is std's, and that
  is the check of last resort there; Kotlin needs nothing.
* [runtime-features] **Which runtime services ship is core's call** (user
  decision 2026-10-05, built 2026-10-06): `salvo_core::features` derives, from
  the emitted modules' declarations and the checker's tables, whether the
  program needs the scheduler (an actor effect or a handler of one; a spawn,
  send, reply, wait or `use` of an addr; an `Addr`, `Pool` or `Reply` named
  anywhere; `pool`/`thread`) and the wire codecs (an actor effect with a wire
  form, a struct with one, a `Reply`). Time travels with either. The set is
  an over-approximation: a service too many ships a file nothing calls, one
  too few would leave a call unbound. Backends keep only their own
  target-specific needs (Rust's string and sequence helpers, Kotlin's throw
  signal, byte buffer and comparison runtime).
* [runtime-sched] **The scheduler is Salvo** (runtime steps, steps 1–15,
  complete 2026-10-03): the core `std/runtime.sv`, its services under
  `std/runtime/` (`timers`, `routing`, `streams`), and the host's part —
  `RuntimeHost` [runtime-host], the platform types and fns in
  `std/platform/runtime.{rs,kt}` and `std/platform/runtime/*` — is the whole
  of it. Each backend's `runtime/scheduler.*` is the entry points generated
  code calls, as shims onto the core, plus the host's registry of proxies.
  Driven by `std/runtime.test.sv`. The pieces: the `Scheduler` monitor (an actor table of `linear struct
  ActorRec canbe Mut` records, each with a `Slot<Body>` and a `Mut
  Deque<Dyn>` mailbox), pools of worker threads that park when idle, send
  with back-pressure (a sender parks on a full mailbox, woken by the next
  dequeue), activations in the fault boundary, and death (the mailbox
  dropped, later sends the no-op); since the second slice, answers — a
  linear `Token` aimed at an actor's continuation (optionally **gated**, so
  only the awaited answer is delivered), at a waiting frame, or at a task
  whose body travels in the token — the `waitfor` bridge (`waiter`,
  `await_answer`) serving its own pool while it waits, `main`'s pool 0 with
  no thread of its own, and the thread-local `here` (pool, actor) a wait
  reads to know what it must not serve. A runtime platform fn's kept
  callback may carry state of its own: the [iter-mut-param] callback rule
  does not apply to it, since an actor body is exactly that. Third slice:
  watches (`Exit` answered at death, or at once for the already dead), the
  fault sink (an unwatched death reported to the pool's sink as a kind-2
  activation carrying a `Fault`, or named on stderr), owed-token accounting
  and `on_idle`, outside sources (`external_begin`/`end`), the deadlock
  report and the main-pool wedge report — the same texts the hosts print.
  Host types: `Dyn` (an erased value), `Body` (an activation,
  `activate(b, msg) -> Ran`), `Slot<T>` (a cell a linear value is taken
  from and put back into through `Mut`). A program's own `Token` or `Body`
  is a different type from the runtime's [type-identity].
  * **The cutover** (2026-10-03): generated code runs on this scheduler.
    Each backend's `scheduler.*` keeps the entry points generated code calls,
    as shims onto the core; the routing layer (proxies, credits, frames)
    moved to the service `runtime.routing` the same day [addr-routable].
    The core calls back `granted(addr, from)` when
    a message that came over the wire leaves a mailbox and `flush_frames()`
    before an activation. A watch is answered with `core.actor`'s `Exit` and
    an idle hook with its `Idle`, built by the core. The runtime module is
    reached by any program using an actor form (by source text outside
    comments, an over-approximation), and an emitter refuses a build that
    ships the scheduler without it. Node ids are non-negative, since the
    core marks a local sender as `-1`.
  * **Finding work** (2026-10-03): each pool keeps a ready queue of actors
    with an entry they may be activated for, in the order they became so; a
    taker drops a stale entry, and an actor is queued again when it next has
    something to run. One idle thread is woken per new item, at once, from
    wherever the item was made (user decision 2026-10-03: deferring the wake
    of a send to the sender's own pool was measured and reverted, since an
    actor's scheduling would then depend on a placement decided elsewhere;
    ROADMAP 0e). An actor's own next entry, which could not run while it
    did, is taken by the thread that ran it.
* [mod-use] **A module-level `use H()` binds an effect for every function of
  its module**, without any of them declaring it (user decision 2026-10-02,
  runtime E4): bound once, on first use, for the life of the process, and
  outermost — a function's own declaration of the same effect shadows it. It
  is how code declared `[]` (`send(reply, v)`, an addr's codec, a host
  thread) reaches the one scheduler. **The runtime module's alone**
  [mod-std-internal]: anywhere else it is refused, since it would be state
  every function reaches without declaring it — hidden state by another
  name. Widening it is a separate decision.
  * Checked as a `use` in a body with nothing in scope (no locals, no
    effects), so its construction depends on nothing the module's functions
    could see; one face per binding for now.
  * Lowering: [rs-mod-use] (an accessor over a `OnceLock`) and [kt-mod-use]
    (a `private val … by lazy`).
* [runtime-host] **`RuntimeHost` is what only the host can do**, one effect
  with a `threadsafe platform handler HostRuntime` per backend in std's
  platform root, bound by the runtime module's `use HostRuntime()`
  [mod-use] (the runtime record). Its members: `secure_bits` (OS entropy:
  `/dev/urandom` on Rust, `SecureRandom` on Kotlin [addr-capability]),
  `report` (a line on stderr) and `mono_nanos` (the monotonic clock). Waiter
  records are reused once their wait has ended, checked by the token's slot,
  so the table holds as many as the most waits ever open at once. What
  else the runtime needs from the host is a platform type or fn of its own
  module: `Parker`, `start_thread`, `guarded`, `Dyn`, `Body`, `Slot`, the
  `here` thread-local, `exit_process`, and the services' `HostIn`/`HostOut`
  and frame codecs. On the virtual runtime [test-actor] the clock and the
  random bits come from the scheduler instead.
* [runtime-parker] **`runtime.Parker` is one thread's park/unpark token**
  (user decision 2026-10-02, runtime E3), a `threadsafe platform type`
  with `this_parker()`, `park(p)`, `park_nanos(p, n)` and `unpark(p)`,
  implemented in std's platform root over `std::thread::park`/`unpark` and
  `LockSupport`. An `unpark` before the `park` makes the next `park` return at
  once — one token, not a count — which is what lets a scheduler record a
  parker in its state and park outside its lock without losing a wakeup; a
  park may also return spuriously, so a waiter loops. Parking on another
  thread's parker traps. Private to the runtime module [mod-std-internal].
  On Kotlin the token is the parker's own flag rather than `LockSupport`'s
  permit, which every JDK lock parks through too (a `ReentrantLock` waited on
  between an unpark and the park consumed the unpark, and the park never
  returned; found 2026-10-03).
  * Found building it: a std tree on disk that shadows the embedded one
    ([std-shadow], `salvo test --src std`) brought its platform files *beside*
    the embedded copies, and the duplicate was refused as a collision; the
    shadowing copy now replaces the embedded one at the same output path.
  * The test harness may import `runtime.test`, since it runs std's annexes.
* [mod-import-module] `import time` imports a whole **module** — every name
  in it (user decision 2026-09-18, with `core.time` moved out to module
  `time`: a std surface that is not implicitly visible needs one line to
  reach, not one line per name).
  * **One module, not the tree under it** (user decision 2026-09-26):
    `import std.fs` brings `std.fs` and `std.fs.mem` takes a second line.
    Importing *less* is what an import is for, and importing more is always
    one more statement — so the reading that can be widened by the writer is
    the default. This reverses the prefix sweep the rule shipped with; a path
    that names no module but has modules under it is refused, naming each one
    as `import <module>`.
  * **The reading is decided by the path**, not by new syntax: every
    segment lowercase *and* a module matching exactly means the module
    form, since a type is uppercase [name-casing] and a fn import still
    has a module prefix in front of it. A single-segment path can only be
    a module, so an unknown one says so and lists the importable modules
    rather than reporting the `module.item` shape.
  * `import core` names no module of its own (`core.list`, `core.string` and
    the rest are separate modules) and is **redundant** rather than unknown:
    everything under `core` is visible anyway [mod-visibility].
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
  `proj(p)`) but visually distinct from user qualifiers, the way
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
* [fn-emit-name] **Emitted names are deterministic and local to the module**
  (user decision 2026-10-05). A fn whose name is unique in its module keeps
  it; overloads within one module are named `<name>__<suffix>` from their
  parameter types, at the first level that sets each apart from the others of
  its name there: (1) each explicit parameter's base type (`next__StrYield`,
  `map__List_Fn`); (2) with qualifiers (`swap__MutList_IdxInt_IdxInt`);
  (3) with type arguments (`get__ListT_Int`); (4) with both; (5) plus the
  implicit parameters' names. Parameters join with `_`, a generated type's
  leading underscores drop, and a fn with no explicit parameters has the bare
  name. Effect members overloaded within an effect follow the same rule
  [effect-member-overload]. **No positional numbers**: overloads no level
  tells apart are a backend error naming them, so a rule can be designed for
  the case. Nothing outside the module takes part, so a name never changes
  because another module — std or the program — gained a declaration.
  Imports are one per name ([rs-imports], [kt-imports]); a file that calls
  fns of one emitted name from two modules, or from another module and its
  own, imports the foreign one as `<name>__<module>`.
  `salvo_core::naming` is the one definition both backends use.
  * Replaced (same day) a program-wide rule: every overload of a name across
    the program was numbered in declaration order (`add__3`, `next__20`), so
    adding a fn anywhere renamed fns elsewhere, and every module imported
    every other by glob.
* [name-camel] **Two value names that would be spelled alike in camel case
  are an error, on every backend** (user decision 2026-10-01, ABI D6): the
  Kotlin backend writes camel case [kt-camel], and a project's validity must
  not depend on its target, so the rule is the language's. The mapping is
  `salvo_core::case::camel`: an `_` before a lowercase ASCII letter is
  dropped and the letter uppercased (`read_to_str` → `readToStr`); a name not
  starting with a lowercase letter, the part from the first `__`, and an `_`
  before a digit, an uppercase letter or the end are kept — so the only
  clashes are spellings like `foo_bar` beside `fooBar`.
  * Checked per Kotlin scope (`case::module_clashes`, from `check_module`): a
    module's top-level fns; a struct's fields; an effect's members; a
    handler's constructor parameters with its state, and its members; and
    each fn with its parameters and the names its body binds (`let`, `for`,
    `is`/`when` bindings, lambda parameters). The error names both spellings.
  * To be loosened if it proves painful (the user's call when deciding).
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
  `is` checks, `+Q` constructors, `canbe` clauses, deduction clauses,
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
    same spelling [rs-fn-mangling] [fn-emit-name], so the ban is
    checked against the whole scope — own module (all its files),
    `core.*`, and imports — not just the declaring file. No encoding
    escapes this: Rust identifiers are `[A-Za-z0-9_]`, so every encoding
    is also a legal name.
  * The name is carried as *one dotted string*, so scope keys, checker
    types, and `ty_base_name` / `type_base_name` agree by construction.
    Backends translate at the point
    a Salvo name becomes target syntax: Kotlin renders it verbatim (a
    valid nested reference), Rust flattens it in `rs_ident`.
  * **A `type` may be the namespace** (user decision 2026-09-29, the aws
    design's 16; ROADMAP §4b item 5): `type StorageClass =
    StorageClass.Standard | …` with `struct StorageClass.Standard {}` beside
    it — how a generated enum keeps its cases apart from another enum's
    `Standard`. Same rules: same file, not generic, two segments, the
    concatenation free; an `intrinsic type` has no body and cannot be one.
    Importing the type brings its members, by the same name-prefix rule
    [name-dot-import]. Kotlin nests the members in an `object StorageClass`
    [kt-nested-dot-name] (the type itself is expanded structurally, so the
    name is free); Rust concatenates as for a struct.
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
* [type-identity] **A type is its declaration, not its name** (user decision
  2026-10-03, option A): two modules may declare one name, and a program may
  use both so long as it never brings them into one scope unrenamed. Every
  struct, effect, alias, intrinsic or platform type and handler has a
  **key** (`typekey`): its name when no other module declares the name, and
  `Name§module` for every declaration of a clashing name but the first —
  std's come first, so std keeps its plain names, which the emitters and the
  hosts name directly. `Ty::Named` carries the key of the declaration the
  resolver chose, the program-wide tables in `Symbols` are keyed by it, and
  a written name is resolved in the scope of **the file that wrote it**
  (`Resolution::type_ref_files`), not of the file where it is lowered — a
  callee's signature, a field, an alias body. So `import lib.b.Token as
  BToken` is `lib.b`'s `Token` wherever it flows, and a `Token` from one
  module is refused where another's is expected.
  * A key never reaches a user: diagnostics print the written name.
  * The emitters render a clashing name with its module's path (Rust
    `crate::lib_b::Token`, Kotlin `salvo.lib.b.Token`; Kotlin leaves a
    module's own declaration bare, since it shadows star imports) and name
    everything derived from it (`__Mon_E`, `__Msg_E`, a dependency field)
    after the written name.
  * An emitter finds a written reference's key by its address
    (`Checked::type_ref_keys`), by its span for a copy, and a name written
    in an expression (`use H()`) through the file's visible names
    (`Checked::visible_keys`).
  * A `by auto` stamp reads the stamping module's own declaration of a name
    first [comptime-instantiate].
* [mod-used-only] Only modules used by the program are transpiled. Roots are
  the user modules declaring `fn main` (all user modules for a library compile
  without one), and from there a module is reached **two ways** (user decision
  2026-09-25):
  * **By resolution, for functions** (`resolved_dep_files`): the checker
    recorded which declaration every call [call-resolve], fn-name reference
    [fn-ref-table], filled implicit [implicit-resolve], resolved comparison
    [cmp-groups], carried identity [cmp-carry], interpolated `to_str`
    [interp-to-str] and driven `next`/`iter` [iter-resolve] resolved to, so the
    edge follows *that*. Both backends' import computation reads the same
    table, so what a file depends on cannot be answered two ways.
  * **By name, for everything else**: a type, struct, effect, handler,
    qualifier or `params` group a file mentions is looked up in its resolved
    scope (`ModuleScope::name_origins`) and each declaring module becomes
    reachable. These are not overload sets, so the conservatism is cheap. A
    name that is *both* a function and a type somewhere in scope keeps its
    name edge — only the pure-fn case is precise enough to narrow.
  * Why the split: a *name* resolves to every module declaring it, so under the
    old name-only rule a program that merely iterated — using the name `next` —
    pulled in `core.range` and whatever it dragged, because `core.range`
    declares a `next` too. The ten checked-in examples lost **11,562 lines** of
    generated code to the narrowing, with nothing added.
  * The two errors point opposite ways, which is why the fn side draws on every
    resolution table rather than only on calls: an edge too many emits a module
    nothing calls (wasteful), an edge too few emits a call with nothing to bind
    to (broken — rustc E0425, an unresolved reference on Kotlin; the shape a
    *missing* resolution edge produced before the tables were consulted at
    all).
  * A module's backend companion files [backend-companion] travel with it;
    a reachable module is emitted only if it produces code.

## Testing

Built 2026-09-23 as the testing MVP (user decisions of that date; the option
space and the rounds that settled it are in COMPLETED.md's decision log, which
replaced the working document TESTING.md).

* [test-decl] `test "an empty heap pops nothing" { … }` declares a **test**: a
  top-level declaration with a string-literal name and a block body. No
  parameters, no effect list, no return type, and `export test` is refused — a
  test is run, never referenced.
  * `test` is **contextual**, like `iter fn` and `send fn`: an identifier
    followed by a string literal at item level, so `test` stays an ordinary
    name everywhere else (`std.test` included).
  * The name is a **plain literal**: interpolation is a parse error, because
    `--list` and the filter have to know a test's name without running
    anything.
  * [test-unique] Two tests in one file may not share a name — the name is the
    test's identity to the runner.
* [random-default] **`random.DefaultRandom` is Salvo over the runtime**
  (2026-10-04, ROADMAP 0.5): a handler whose `random()` is
  `runtime.random_double()`, two draws of the scheduler's Lehmer generator
  as one double in [0, 1). Seeded by OS entropy on first use, and by the test's
  seed in an actor test, so its draws repeat there [test-actor]. Not
  cryptographic. `core.console`'s `StdOutConsole` is a `threadsafe platform
  handler` with its host files in `std/platform/core/console.{rs,kt}`; std
  declares no `intrinsic handler` any more.
* [test-kind] **A test's kind decides what it runs on** (user decision
  2026-10-03, D11): `test "…"` is a plain test, on the threaded runtime;
  `test actor(ARGS) "…"` is an actor test, on the virtual runtime
  [test-actor]; `test property(ARGS) "…"` is planned. The kind and its
  arguments come before the name; with no arguments the parentheses may be
  left off. `salvo test` writes **one program per runtime** — the plain tests,
  then the actor tests — since a process cannot leave the threaded runtime
  once its workers have started.
* [test-actor] **An actor test** declares `[use, spawn, Throw<Failure>]` and
  runs on the **virtual runtime**, which the harness enters before each one
  (`test.actor.begin_actor_test(seed)`): no worker threads (the test's frame
  serves every pool while it waits); a clock the scheduler moves to the
  earliest deadline when nothing can run, starting at 0 (`time.monotonic_nanos`
  reads it, and the timer service arms a clock hook instead of its wheel);
  randomness from the seed (`seed: N`, default 0; a Lehmer generator); a
  fresh scheduler (every earlier actor dead without a report, queued work
  dropped, pools retired, pending deadlines forgotten); a send to a full
  mailbox runs work until there is room; and a wait nothing can answer is the
  deadlock report, since a park could never be woken. Opening a host thread
  (`external_begin`) stops the program with a report. The harness does not
  wrap an actor test in `trapped_by` (a lambda cannot spawn
  [actor-spawn-expr]), so a trap in the test's own frame is a death the
  runner recovers from [test-recover].
* [test-file] A test lives in a **test annex** and nowhere else: `heap.test.sv`
  beside `heap.sv`. A `test` block in a production source file is an error
  naming the annex as the fix (`salvo_core::expand`, before resolution, so
  every driver reports it).
  * `.test` is the **only** dot a `.sv` file name may contain
    ([mod-file-name]'s carve-out, deliberately narrow so the per-backend
    companion spelling `string.kotlin.sv` stays dead). The file is module
    `<name>.test`.
  * An annex is loaded by `salvo test` and by `salvo analyze` (and so by the
    language server) — **not** by `compile` or `run`. That is the whole of how
    tests stay out of a production build: there is nothing to strip.
  * An annex whose module has no production file is an error naming the orphan;
    so is `heap.test.sv` beside a `heap/test.sv`, since the two spell one
    module path.
  * An annex is **never `is_std`**, whatever tree it lives in, so a test cannot
    declare an `intrinsic` [intrinsic-std-only] (user decision 2026-09-23).
* [test-visibility] An annex sees the module it tests **whole**, private
  declarations included, at the same rung as the module's own declarations
  (`Level::Own`, so overload ranking in a test matches the module's
  [fn-overload-scope]). The reverse never holds — the annex is its own module,
  nothing imports it, and a production build does not load it — so the
  one-way visibility is by construction rather than by a check.
  * An annex may declare its own helpers, structs and qualifiers. They are
    invisible to the module under test, which is what keeps test vocabulary out
    of a shipped surface.
* [test-implicit-import] Every annex behaves as if it wrote a whole-module
  import of **`std.test`** (module `test` in the embedded tree): `expect`,
  `expect_eq` and `Failure` are in scope without an import line, at the
  bulk-import rung [mod-import-module] — so a file's own `expect` silently wins
  over the standard one.
* [test-body] A test body has an entry point's powers: `use` is available with
  nothing declared, exactly as in `main() [use]`, which is how a test registers
  a fake (`use MemFs()`). Everything else about the body is the language
  unchanged — narrowing, linearity, deductions, effects.
  * `spawn` is *not* in the MVP's implicit powers; actor testing is the slice
    after it (ROADMAP.md).
* [test-fail] Failure travels on the existing non-resumption channel
  [throw] [try] (user decision 2026-09-23, TF-3(i) — no second channel, and no
  `Test` effect to intercept): an assertion declares `[Throw<Failure>]`, and
  `Failure` is a std struct with a `message: Str` and a `to_str`.
  * A test therefore **stops at its first failing assertion**. Several
    independent facts are several tests.
  * A custom assertion is an ordinary function declaring `[Throw<Failure>]`;
    there is nothing to register.
  * `expect(condition: Bool, label: Str)` is the general assertion.
    `expect_eq<T>(actual: T, expected: T, ?Eq<T>, ?ToStr<T>)` adds equality and
    rendering as **capabilities** [cmp-groups] [interp-to-str], so a type joins
    in by declaring the two functions; a type that declares neither gets the
    ordinary implicit-resolution error at the assertion site.
  * std gained `to_str` for the scalars with this rule (`Int`, `Long`, `Byte`,
    `Char`, `Bool`, `Str`): interpolation renders them natively, but a
    `?ToStr<T>` position needs a *function* to resolve [implicit-resolve].
    `Double`/`Float` have theirs since 2026-10-04, by [interp-float], so a
    `List<Double>` prints through `?to_str` [platform-value-type].
* [test-run] A `test` block is expanded **before resolution**
  (`desugar::expand_tests`, run from `salvo_core::expand`) into an exported,
  parameterless fn named `__salvo_test_<module_mangled>_<index>` declaring
  `[use, Throw<Failure>]` with the block as its body. Nothing downstream knows
  the form exists: resolution, the checker, the deduction pass, both emitters
  and the LSP see an ordinary function.
  * `salvo test` then **synthesizes one Salvo module** (`__salvo_test_main`),
    added to the source set rather than written to disk, whose `main` calls
    each selected test inside its own `try` and reads `Ok … | Thrown Failure`
    off it. It is compiled and run exactly as `salvo run` compiles and runs a
    program (user decision 2026-09-23, TF-4(a)), so **no emitter has a line of
    test-shaped code** and the two backends agree by construction.
  * The harness prints a **protocol** — `##salvo-test begin <id>`,
    `##salvo-test ok`, `##salvo-test fail <message>` — and the runner renders
    the report. Two reasons: the report is coloured, and a Salvo string literal
    has no escape for the ESC byte; and per-test milliseconds come from timing
    the protocol lines as they arrive, so a test needs no clock capability.
  * A test that begins and never reports (a panic, a killed process) is
    reported as `DIED` and named in the summary: a run that ends silently is
    the one failure a report must not lose.
* [test-trap-expect] `std.test`'s vocabulary for a test *about* a trap, over the
  same catch the harness uses [test-recover]:
  * `trap_of(body: () -> None) -> Str?` — the trap's message, or `None` when the
    body completed.
  * `expect_trap(body, label)` — fails the test unless the body traps.
  * `expect_trap_with(body, needle, label)` — fails unless it traps with a
    message *containing* `needle`, which is how a test pins which failure it
    meant rather than accepting any.
  * The body is a **pure** fn value, so it cannot inherit an effect from the
    enclosing test: a body that needs one **registers it itself**
    (`() -> { use StdOutConsole(); … }`), which is legal because a lambda inside
    a test inherits the test's own `use` permission [test-body]. Effect
    polymorphism would remove the ceremony and does not exist (ROADMAP.md).
  * These are ordinary functions that `throw` [test-fail], so a test *about*
    them reads its own failure off a `try` — which is what `std/test.test.sv`,
    the test module's own annex, does.
* [test-recover] (`std.test`'s `trapped_by` is Salvo since 2026-10-04: a call
  of `runtime.trap_boundary`, the runtime's fault boundary for a lent body,
  rather than an intrinsic each backend lowered.)
  A test that **traps** is that test's failure, not the end of the
  run (user decision 2026-09-23, A-5): a failed `assert!` [assert-trap], a
  subscript out of range, any failure the program is not meant to continue past
  is caught by the **generated harness** and reported like any other failure.
  * The catch is `std.test`'s `trapped_by(body: () -> Str?) -> Str?`, an
    `intrinsic` lowered to each host's own catch — an exception handler on the
    JVM [kt-assert-trap], `catch_unwind` on Rust [rs-assert-trap]. No language
    surface: catching a trap is something *std* can reach because `intrinsic` is
    how std reaches a host, and it stays confined to test files because
    `std.test` does [test-implicit-import] — production code still cannot catch
    a trap, which is the stance A-2 took.
  * The harness body **answers** its failure rather than printing it, and
    performs no effects, so nothing has to be threaded into the catch: `try`
    reads the declared failure [test-fail] and hands it back as the body's
    value, `trapped_by` answers the trap instead, and the verdict is printed
    outside. On Rust the panic hook is silenced for the duration, so the report
    is the only place the failure appears.
  * **Catching beats restarting**, which is why this replaced the first
    implementation the same day: a restart cannot be coordinated across tests
    running *concurrently*, and parallel tests are where this is going. The
    runner keeps a restart path as a **backstop** for a death the harness cannot
    catch (a process killed outright): the test in flight is named, whatever it
    left on stderr is printed under it, and the remainder re-runs in a fresh
    process.
* [test-report] A test's **id** is `<module under test> :: <name>`: the module
  a program would `import` (`heap`, not the annex's `heap.test`) and the name
  without its quotes (user decision 2026-09-23). The report is one line per
  test — `ok` green with its milliseconds, `FAILED` red with the failure
  indented under it — then a blank line and a count. Output from the code under
  test is passed through, never swallowed. Under `backend = "*"` the suite runs
  once per backend, each report headed by the backend's name (`rust:`).
* [test-filter] `salvo test --src DIR [--backend B] [FILTER] [--list]
  [--target DIR] [--clean-target before|both]`.
  `FILTER` is a plain substring of the id, so one word selects a module, a
  test, or a family; `--list` enumerates and runs nothing. The command's exit
  code is nonzero iff something failed. Colour is on when stdout is a
  terminal.
  * `--backend` defaults to `rust`, as `salvo run`'s does [cli-run].
  * `--target` defaults to `.salvo_tmp_test`, and `--clean-target` says what
    survives, exactly as it does for `run` (user decision 2026-09-26). The
    default differs, and deliberately: `before`, so the generated harness is
    still there to read after a failure. `both` deletes it after the report.
* [manifest] **A project is described by `salvo.toml`** in its root (user
  decisions 2026-09-24 for the direction, 2026-09-29 for the shape; ROADMAP
  §4 built the same day): `[project] name`, `version`, and `std = true` for the
  standard library's own tree; `[build] src` (the source root, relative to the
  manifest, default `.`), `main` (the entry file, when there are several),
  `backend` (`rust`, `kotlin`, or `*` for every backend), `target` (one
  output directory for all backends), `modules` (where dependencies live
  [manifest-deps]) and `platform` (where platform files live, required when
  there are any [platform-root]); `[rust]`/`[kotlin]` sections each with a
  `target` and a `platform` that override them; `[dependencies]` [manifest-deps]. Every command reads it — `run`, `compile`,
  `test`, `analyze`, `platform generate`, the language server — with one
  precedence: a CLI flag, then the manifest, then the built-in default. Under
  `backend = "*"` a command runs for each backend in turn (a `compile` into
  each backend's own `target`; a `run`/`test` into a per-backend scratch
  directory), which is how every example regenerates both trees with a bare
  `salvo compile`. A `main` is optional: without one, `analyze` and `test`
  work and `run` says there is nothing to run. Unknown keys and an unknown
  backend value are refused naming the file. `run`/`test` never read the
  manifest's `target`: they clean their directory, and the manifest's is where
  `compile` keeps the checked-in output.
* [manifest-discovery] **A file belongs to the nearest ancestor directory
  holding `salvo.toml`**; a nested manifest is a boundary, so a parent
  project's source walk does not enter it. The CLI discovers from `--src`,
  else from `--main`'s directory, else from the working directory; with no
  manifest above, it needs `--src` or `--main` as before. The language server
  discovers **per document**, analysing each open document under its own
  project — one analysis per project root — so a workspace holding several
  projects (this repository: `std/`, every `examples/*/`, `demo/`) checks each
  as its own program. A document with no manifest above it is analysed under
  the workspace root, as before, and — in a workspace that does hold projects
  — carries an information diagnostic saying so.
* [protocol-lock] **`salvo.lock` beside the manifest locks the protocol hash of
  every actor effect the project declares** (the network round's N-9, 2026-09-26,
  shaped 2026-09-29): `version = "…"` and `[protocols] Effect = "hash"`, for the
  project's own `actor effect`s with a wire form (never std's). Every build
  reconciles it before emission: absent, or the manifest's `version` changed,
  it is written; a hash that changed **at the same version** is an error
  naming the effect and both hashes, with the remedy "bump `[project] version`"
  — there is no `--relock`, since a silent protocol change is what the file
  exists to stop. A protocol added or removed relocks without error. Std
  shares the predicate (`effect_has_wire_form`) with both emitters, so what is
  locked is exactly what carries a `__PROTO_E` constant [protocol-hash].
* [std-shadow] (`std/salvo.toml` says `std = true` [manifest], which marks the
  whole tree std — a new module the embedded copy lacks included — and is what
  makes the repository root openable in an editor.) A source tree may **replace** modules of the embedded standard
  library: every module a loaded file declares that an embedded std file also
  declares drops the embedded copy, and the disk file takes over with its
  std-ness (so its `intrinsic` declarations stay legal). This is what
  `salvo test --src std` rests on — it tests the checkout, not the std
  compiled into the binary — and without it the two copies would collide as
  duplicate declarations [mod-collision].
* [manifest-deps] **A project's dependencies are other projects, named under
  `[dependencies]` and found by name under `[build] modules`** (user decisions
  2026-09-29; the 2026-09-29 manifest entry had parked "dependencies between
  projects" as out of scope, and this is the first slice). `aws = "0.1.0"`
  means `<modules>/aws/salvo.toml` exists and its `[project] version` is
  `0.1.0`; a missing directory, a version mismatch, or a `[dependencies]`
  table without `modules` is an error naming the manifest. A directory under
  `modules` that no entry names is not loaded. Loaded **between the embedded
  std and the project's own tree** (`analysis::load_dependencies`,
  `SourceSet::add_dependency`), by every command and the language server.
  * **No prefix**: a dependency's module paths come from its own layout
    exactly as std's and the project's do (`salvo/aws.sv` is `aws`;
    `salvo/aws/s3.sv` is `aws.s3`) — one path rule, and the dependency's
    author owns its namespace by its tree. Two files declaring one path are
    the ordinary [mod-collision].
  * **Std-ness follows the dependency's own manifest.** A dependency
    declaring a module the embedded std also declares is refused, naming the
    file and the fix (`[project] std = true` in *its* manifest) — a library
    must not redefine `core.list` on its users. With `std = true` it *is* a
    standard library: its files are std and replace the embedded copies they
    name, as [std-shadow] lets a tree do. `mark_std_tree` and
    `apply_std_shadow` leave dependency files alone; `add_dependency` has
    already settled them.
  * A dependency's files are tagged (`SourceFile.dependency: Some(name)`),
    named by **absolute path** (they sit outside the source root, so a
    root-relative name would lie), and loaded with tests off. Its `main` is
    never an entry point [cli-run]; its actor protocols are not the project's
    to lock [protocol-lock]; `salvo platform generate` writes no skeleton for
    its platform handlers — but its **companions are loaded** (host files under
    its platform root, user decision 2026-09-29): a dependency's platform handlers
    need their implementations as much as the project's do.
  * Hover: a declaration reached from a dependency says ``From `m` —
    dependency `name`.`` [lsp-fn-origin]; a module from one says so too
    [doc-module]. In the editor a file *inside* a dependency belongs to its
    own manifest [manifest-discovery] and is analysed as its own project.
  * **Not yet**: transitive dependencies (a dependency's `[dependencies]` are
    not followed), version *ranges* or resolution, a fetcher filling
    `salvo_modules` from anywhere, and a check that a dependency's own tree
    does not need a `[build] modules` of its own. The repository's first
    dependency is `modules/aws/` (`ProfileCredentials`, the services to
    follow as actors), consumed by `examples/aws_profile/` through `modules =
    "../../modules"` — the setting pointing anywhere is what makes
    `salvo_modules` a convention rather than a rule.

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
  `platform handler` [platform-handler]. Applies to
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
* [platform-effect] **Removed 2026-10-01** (user decision; the ABI decisions). A
  `platform effect` — an effect implemented wholly by the host, whose instance
  was handed to `main` by a host-owned entry point (`salvoMain` /
  `salvo_main`) — is no longer the language. `platform` takes `handler` only,
  and the parse error says so. No shipped source used one; a platform handler
  of an ordinary effect [platform-handler] covers the same ground, and the
  form may return when a need for it appears.
* [platform-handler] `platform handler H of E` declares a handler of an
  **ordinary** Salvo effect whose implementation is a class the *build*
  supplies, in the target language (FS-1 resolved as O-M2, user decision
  2026-09-14; the deferred 2026-09-05 proposal, un-deferred because
  phase 4's `HostRawFs` needs it). The effect stays Salvo's, with as many
  other handlers as it likes; only this one is host code. It is the one
  interop path customer code has.
  * It is registered with `use` like any handler — that is the point of the
    form — so **the entry point does not move**: the instance is
    constructed inside the program, not handed to it. Constructor
    parameters are passed through to the host class
    (`use HostS3("bucket")`).
  * Under [effect-handle] a platform handler binds as a handle — a
    monitor unless it declares `threadsafe` [threadsafe-platform], bare when
    it does — so handlers depending on its effect are written the same
    whichever the host declared.
  * **The host implements a host-facing interface, and the program reaches
    it through an adapter** (ABI D7, D8; 2026-10-01). Beside an effect
    some platform handler implements, the compiler emits the interface the
    host class implements (`EPlatform`; on Rust `EPlatform` with `&mut self`,
    or `EPlatformSync` with `&self` for a `threadsafe` handler) and the
    adapter `__Platform_E`, which implements the effect for the program by
    forwarding each member — the place host→Salvo values are checked. The
    declaration emits `__Platform_H`, the adapter with the handler's
    constructor, which builds the host's class; a `use` constructs that. The
    class is named after the **handler**. Emitted only where the host file
    exists (or in the host project), so a handler nobody uses needs none.
    Not generic effects, and no member returning a borrow (codegen errors).
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
    host writes one concrete class).
* [platform-type] **`platform type Name` is an opaque handle to a host
  object** (user decision 2026-10-02, the runtime record): Salvo sees a name and
  modifiers, never contents; the host implements a class (Kotlin) or struct
  (Rust) of the **same name** in the declaring module's implementation file
  [platform-tree], and `salvo platform generate` writes its skeleton.
  Operations are `platform fn`s and platform handler members taking or
  answering it, reached with dot notation like any fn; there is no member
  syntax on the type.
  * **Three kinds**, because copying has to mean one thing: *plain* — a copy
    shares the host object (Rust: `Clone`, expected cheap, an `Arc` inside);
    **`threadsafe platform type`** — the same, usable from several threads
    at once (Rust adds `Sync`); **`linear platform type`** — exactly-once,
    never copied, owed until a platform fn consuming it (`=> !c`), and the
    only kind that may be `canbe Mut`, so a `Mut Cursor` parameter is
    `&mut Cursor` on Rust. `threadsafe linear` is refused.
  * [platform-value-type] **A copyable `platform type … canbe Mut` is a
    value type** (user decisions 2026-10-04, ROADMAP 0.7, convention (c)):
    a copy is the host's own deep copy, never a shared object, so `Mut`
    means what it means for a struct; `threadsafe` refuses it. The host
    names the mutable kind **`Mut<Name>`** beside `Name` (Kotlin renders `Mut
    Bytes` as `…MutBytes`; Rust may alias one type for both, passing `Mut` as
    `&mut`), and Kotlin's host package provides `copy(x: Name)`, which
    generated code calls wherever Salvo copies one. Rust asserts `Clone`
    (deep), `Debug`, `PartialEq`, `Eq` and `Hash`, so a struct holding one
    still derives. A value type keeps its canonical wire form where the
    encoding defines one [wire-format] (`Bytes`); otherwise it is `noremote`.
    Every collection is built: `Bytes`, `Str`, `Deque`, `List`, `Set`, `Map`,
    `SortedSet` and `SortedMap`. A `Map` iterates its keys through its host's `each`
    [col-map-iter], and its `to_str` stays intrinsic: rendering keys *and*
    values in Salvo needs two `to_str`s, and implicits resolve by name.
    On Rust an argument reading a place another argument lends `&mut` is
    hoisted into a `let` [rs-borrows], since a free fn gets no two-phase
    borrow (`put(m, k, size(m))`). `list_of`/`mut_list_of` stay intrinsic, since literals and
    spreads lower through them.
    For the collections (user decisions 2026-10-04): **std's own platform
    fns may answer a borrow** of the one parameter the result names
    (`get(d, i) -> (proj(d) T)?`), which customer code's still may not;
    **`to_str` of a collection is Salvo** over a `?to_str: (x: T) -> Str`
    implicit, so a collection prints only where its element does; and
    **`drain` is Salvo** over `into_mut`, `remove_first` and `end_empty` (a
    host fn that traps on a non-empty container), so no effectful callback
    crosses the boundary. A `core` platform fn's Rust type parameters are
    bounded `Clone` for a plain `T` and not at all for a `T canbe linear` —
    an element may then be a borrow — and a struct's carry none (the derives
    bound their own impls). std's Rust wrappers take a lent fn value as
    `&mut dyn FnMut`, as their hosts do, since a caller may already hold it
    that way (an implicit `cmp`). The locator variant of a borrowing platform
    fn [rs-loc] answers the borrow's position: `List`'s `get` its index,
    anything else found by address. `for` over a `List`, `Str` or `Bytes`
    stays the backends' native loop [iter-for-native]. A platform
    fn's Rust wrapper is called through its module's path
    (`crate::core_string::size_platform`), since two modules' wrappers of
    one name are ambiguous through glob imports.
  * [platform-iterable] **`iterable platform type List<T> canbe Mut :
    Iter<self, T>`: a `for` over one is the host's own loop** (user decision
    2026-10-05). The modifier is contextual and only `platform type` may
    follow it; the declaration must also take `: Iter<self, T>`, the one
    obligation a platform type may take (without `by`, and allowed without
    `iterable` too), so Salvo's iterator and the host's loop agree on the
    element type and a generic fn over `Iter` still finds a pass. The host
    provides **`each(x)`**: Kotlin an `Iterable<T>`; Rust an `Iterator` over
    `&T` for a type with parameters and over whatever clones to the element
    otherwise (`Str`'s `chars()`), and for a type with parameters also
    **`each_mut(&mut x)`** over `&mut T` (a loop writing a field of its
    element, [for-elem-write]) and **`into_each(x)`** over owned elements (a
    loop that consumes the container). Rust asserts all three against the
    obligation's element type beside the re-export, and `salvo platform
    generate` writes their stubs. When a module declares more than one
    iterable platform type, each one's fns carry its name in snake case
    (`each_sorted_set`, `each_sorted_set_mut`, `into_each_sorted_set`; Kotlin
    `eachSortedSet`), since one Rust host file holds one `each`
    (`salvo_core::naming::each_suffix`). Every std collection is iterable; a
    by-value loop over an elementless one clones out of `each`.
  * **Always `noremote`** [noremote] — a host object has no wire form, and
    the wire predicate says so for a struct holding one — and
    sendable [actor-sendable] (Rust: `Send + 'static`), so a handle may sit
    in actor state or travel in a local message.
  * [platform-slots] **Identity slots** are allowed on a copyable platform
    type (user decision 2026-10-04): `platform type Set<T>(?hash: (T) -> Long,
    ?eq: (T, T) -> Bool)`, part of the type as on any type declaration
    [cmp-carry]. **The identity is the type's, passed per call** (user
    decision 2026-10-06, ROADMAP §0j step 7, `heap`'s model): every platform
    fn that needs it captures the binders from its parameter's type —
    `platform fn add<T>(set: Mut Set<T>(?hash, ?eq), elem: T)` — and the
    wrapper hands them to the host as trailing fn parameters, lent for the
    call [platform-fn-value]; a constructor takes them as implicits
    (`set_of<T>(...elems: T[], ?Hashed<T>)`). The value stores no fn, so the
    host keeps one representation for every identity: std's `Set`/`Map` are
    an insertion-ordered slab (entries, tombstones, the digest each was filed
    under, a bucket index), the sorted pair a sorted list searched by
    bisection, in `std/platform/core/{set,map,sorted}.{kt,rs}`. A value type
    with slots promises no `Hash` on Rust, since it is not a key
    [col-key-eligible]. `linear` refuses slots.
    * **Consequence for generic code**: a generic fn that looks an element up
      captures the identity too (`fn has<T>(s: Set<T>(?hash, ?eq), e: T)`), as
      a fn over a `Heap` captures its `?cmp`; `Set<T>` with nothing written
      resolves `hash` at `T` by name, which a bare `T` cannot answer.
    * **Host code builds the canonical one** (`canonicalSet`/`canonicalMap`/
      `canonicalSortedSet` on Kotlin, `canonical_set`/`canonical_map`/
      `canonical_sorted_set` on Rust): the identity Salvo resolves for an
      intrinsic key type, which is the host's own hashing and equality
      (`hashCode`/`==`, `DefaultHasher`/`PartialEq`) and code-point order.
      A set or map at the boundary keyed by anything else — a written
      identity, a hand-written `hash`, a `by auto` stamp, whose `hash` folds
      fields its own way — is refused [platform-check].
  * **Refused**: an obligation clause other than `: Iter<self, T>`
    (`by auto` has no fields; comparisons are platform fns), an alias.
  * [platform-generic] **Type parameters are opaque to the host** (2026-10-02,
    runtime E1): `platform type Cell<T canbe linear>` and `platform fn
    erase<T canbe linear>(v: T) -> Dyn`. The host may store a `T`, move it
    and hand it back, nothing more: Rust bounds every parameter `Send +
    'static` (which also gives `Any`, for a downcast) and never `Clone`, so a
    linear argument fits; Kotlin takes a plain type parameter. Answering a
    `T` from a *kept* plain handle would need a copy the host cannot make,
    which is why a by-value reader takes the handle (`take(c: Cell<T>) => !c`).
    A generic *Salvo* fn calling a generic platform fn needs its own `T` to
    be `Send + 'static` on Rust, which a generic fn does not promise: rustc
    refuses the program, recorded in ROADMAP 0c. Kotlin cannot check an
    `unerase<T>` (its `T` is erased), so a mismatch surfaces where the value
    is used — the runtime record
  * Nothing to check at the boundary [platform-check]: the value is opaque.
  * Lowering: [rs-platform-type], [kt-platform-type].
* [platform-fn-value] **Function values cross the platform boundary,
  effect-free, and lent for the call unless named** (2026-10-02, runtime E2;
  widened 2026-10-06, ROADMAP §0j 6h): a platform
  fn or platform handler member may take `(A) -> R` or `once (A) -> R`; the
  host receives its own closure type (Kotlin `(A) -> R`; Rust `impl FnMut` /
  `impl FnOnce` for a fn, `&mut dyn FnMut` / `Box<dyn FnOnce + '_>` for an
  effect member, whose trait is `dyn`). The callback declares no effects (a
  fn type with effects is refused there as elsewhere), and it is **lent for
  the call**: Rust's types forbid keeping it, and on Kotlin keeping it past
  the call is a breach of the host contract, as aliasing a collection is
  (ABI D10 C4). A callback the host keeps and runs on a thread of its own
  is not available to programs — a fn value is not sendable
  [actor-sendable] — and is the runtime module's privilege when the
  scheduler needs it (runtime step 11) [runtime-kept-fn].
  * **A named fn may be kept** (user decision 2026-10-05): a platform fn
    whose clause consumes a plain fn-typed parameter (`=> !hook`; a `once`
    one is still lent, and consumed by its call) **keeps** it, and every
    argument there must be a **named top-level fn** — capture-free, and
    effect-free by its type — the identity restriction [cmp-carry] has. A
    lambda or a local is refused, naming the rule. The host may call it from
    any thread, as often and as late as it likes: Rust's host takes a plain
    `fn(…) -> R` pointer (`Copy + Send + Sync + 'static`; the call site's
    adapter captures nothing and coerces to it), Kotlin's its own fn type.
    What std's `net` uses for its outbound hook (`bind_outbound`).
    `platform_keeps_param` (`salvo_core::check`) is the one predicate the
    checker and the emitters share.
* [platform-never] **A platform fn or platform handler member answering
  `Never` cannot return** (2026-10-02, runtime E6): the host's
  signature says so in its own type system — Kotlin's `Nothing`, Rust's `!`
  (the program's own effect trait keeps `()` and the adapter forwards the
  host's `!`) — so a host body that returns is the host compiler's error,
  never a silent fall-through after a call the checker treated as diverging
  [backend-never-wrong]. What the runtime's `exit_with` needs.
* [platform-check] **What crosses from host to Salvo is checked** (user
  decisions 2026-10-01, ABI D7, D10 C3): a platform fn's wrapper checks
  its result; a platform-handled effect's adapter checks each member's
  result and, through the reply it hands the host (`reply.checked(…)`), the
  value the host sends on a `Reply<T>` — on the sending thread, before
  delivery. A failed check **panics** (Rust) or **throws**
  `IllegalStateException` (Kotlin), naming the declaration and the value.
  * What is checked is what the Salvo type promises and the host type cannot
    say, over the runtime shape [union-arm-identity]: a **closed** literal
    arm (`"gold" | "silver"`; an open one, `| Other Str`, admits any value of
    its base); a **state** qualifier, by running its `qualifies`; and these
    inside lists, sets, arrays, maps, struct fields, tuple elements,
    nullable values and union arms. A **provenance** qualifier is trusted
    (`Ok`, `Err`, `Other` are provenance); a **constructive** one (no
    `qualifies`) is trusted when declared in the platform declaration's
    module — for a handler, the effect's — and refused otherwise. A
    `qualifies` that needs effects or slot values cannot run at the
    boundary and is refused, and so is a recursive type with fields to check.
  * A check that walks a collection **warns** on the declaration, naming the
    cost (D10 C3).
  * The plan is the checker's (`Checked::boundary_checks`,
    `salvo_core::abi::BoundaryCheck`), so both backends check the same
    thing; a host project's adapters carry no checks (it only type-checks
    the implementation files).
  * A platform fn whose result borrows (`proj`) is refused (D10 C5), as a
    platform-handled member's is at emission.
  * **Collections** (D10 C2; revised 2026-10-06, ROADMAP §0j step 7): a
    set or map is std's own host type, which host code builds with the
    canonical builders [platform-slots], so its order is the type's and
    nothing about its shape is checked (the Kotlin shape checks and
    normalizers went with the JVM's collections). One keyed by anything but
    an **intrinsic** identity — written (`SortedSet<Str>(by_len)`), filled
    from a hand-written `hash`/`eq`/`cmp`, or `by auto` stamped — is refused
    in a result, since the host cannot hash or order it Salvo's way.
* [platform-factory] **Factories build a union at the boundary** (user
  decisions 2026-10-01, ABI D5). Positional unions stay; beside them the
  compiler emits one factory per runtime arm for every union a host builds:
  a **named** union reached from any platform signature (`type FsError = …`:
  Kotlin `object FsErrors`, Rust `impl FsError` with a `pub type FsError`),
  and a platform fn's or platform-handled member's **anonymous** result or
  `Reply<T>` payload (an object named after the fn or member, upper camel:
  `ReadToStr`, `GetQueueUrl`). They compose:
  `ReadToStr.err(FsErrors.notFound(NotFound(path)))`.
  * Names, by arm (`abi::factory_name`): a struct by its name (`notFound`),
    a qualified arm by its first qualifier (`ok`, `err`), a base by its own
    (`str`, `int`, `list`); a base's literals share **one** factory of the
    base, which checks its argument when the arm is closed [platform-check];
    `None` has none (the host writes `null` / `None`). A name two arms would
    share gets **no** factory, and a comment says so. `Checked` is an
    ordinary struct.
  * Kotlin spells them in camel case, Rust in snake case. An object named
    like an existing type is a codegen error. Rust emits no factories for a
    named union that admits `None` (no inherent impl on `Option`).
  * Emitted in the build for every module's platform signatures (std's and
    dependencies' implementation files call them too), and in the host
    project.
* [platform-stamp] **A host project is stamped, and a dependency's is checked**
  (user decision 2026-10-01, ABI D9 (b)). Every generated file of a
  platform root carries `salvo-abi <revision> <hash>` in its header: the
  compiler's ABI revision (`abi::ABI_REVISION`, bumped by hand when a change
  would break implementation files written against the old generated code)
  and an FNV-1a hash of the owner's platform signatures and everything they
  reach, rendered canonically (types through `Display`, deduction clauses
  with their spacing collapsed), so moving code or reformatting it does not
  count.
  * A dependency checks in its own host project. A build that compiles a
    dependency's implementation files compares the stamps of the
    dependency's `*.sv.<ext>` files with what it computes, after emitting and
    before the host compiler runs, and refuses a mismatch naming the cause —
    no host project, another ABI revision, or signatures changed since it was
    generated — and `salvo platform generate` in that directory as the fix.
    A build that does not reach the dependency's platform code does not look.
* [threadsafe-platform] `threadsafe platform handler H of E` states the host
  class's **thread-safety contract** (user decision 2026-09-26, closing the
  2026-09-20 "assumed thread-safe" stance): the instance may be entered
  concurrently from any thread, because the host synchronizes internally or
  holds nothing that needs it. The word is contextual (an ordinary identifier
  everywhere but directly before `platform`), a whole-handler claim (no
  per-member form), and legal only on the handler form — before `platform
  effect` it is a parse error saying why (an effect names members, and has no
  instance to be safe or unsafe).
  * **Undeclared = serialized on both backends.** A platform handler without
    the word is **stateful** under [effect-handle]: Rust binds the host in the
    handle's locked arm (`E::locked(H::new(…))`), Kotlin in the effect's
    lock wrapper (`__Mon_E`). A host that did not claim safety therefore behaves
    identically everywhere and pays only the lock — the *safe* default, and
    the one the pre-2026-09-26 Kotlin emission lacked (it bound the raw
    instance, so a non-conforming host raced there and was accidentally
    serialized on Rust).
  * **Declared = shared raw.** A `threadsafe` handler is **stateless** for
    the handle's purposes: Kotlin binds the raw instance; Rust shares it as
    `Arc<dyn __Stateless_E>` — the host implements the effect's `&self`
    trait, so it is compiled under shared access and rustc refuses interior
    mutability that is not `Sync`, the half of the contract a compiler can
    check ([rs-handle]). The Kotlin host stays on trust. (For the afternoon
    of 2026-09-28 the word was emission-neutral, every handler being locked;
    keying the handle on statefulness restored it the same day.)
  * **`salvo platform generate` prints the contract** into the skeleton it
    writes, in both shapes, so the person implementing the host signs what
    the compiler assumes: the threadsafe skeleton's receivers are `&self`
    (Rust) and the comment says there is no lock; the undeclared skeleton
    says the compiler serializes the instance. Regenerating after adding or
    removing the word changes the skeleton's shape.
  * The first customer is the network sequence's `Transport` (ROADMAP.md
    section 2), called from every pool; std's `HostRawFs` keeps plain
    hash-map state on both backends and stays undeclared, so it is now
    serialized on Kotlin as it always was on Rust.
* [platform-root] **Where platform files are read from is the manifest's to
  say, and it has no default** (user decision 2026-09-30): `[build] platform`
  names one directory for every backend, `[kotlin] platform` / `[rust]
  platform` one per backend, over it; paths are relative to the manifest.
  Both backends may share a root — the extensions keep the files apart, as in
  `modules/aws` (`platform = "salvo/platform"`) and `std/`
  (`platform = "platform"`) — or each may have its own, inside the source
  tree or beside it (`kotlin/`, `rust/`).
  * A program with a `platform handler` or a `platform fn`
    fn needs a root for every backend it builds, and for the one being
    built: otherwise it is refused at the first such declaration, naming the
    key (`platform_root_required`, in every build and in `analyze`). A
    program with no manifest has no root and cannot use platform files.
  * A file is attributed by its root (`PlatformRoots::classify`): its path
    under the root, the way `.sv` files map, so `<root>/app/entry.kt` is
    module `app.entry`. `<root>/app/entry.sv.kt` (any `*.sv.kt` / `*.sv.rs`)
    is a generated file for the host project's tooling and is never read by
    the build (ABI D4). A Kotlin
    file under the Rust-only root (or the reverse) is an error; so is a host
    file in a `platform/` directory at the source root that no root covers
    — it is reported, never read as a module called `platform.…`.
  * In the output a host file is always `platform/<module path>.<ext>`
    (`SourceSet::platform_output_path`), whatever root it came from: the
    emitted code names it there.
  * A dependency's roots are its own manifest's; std's are `std/salvo.toml`'s,
    read by the embedded loader as well as from a checkout.
* [platform-tree] The host implementations live in the backend's **platform
  root** [platform-root], mirroring the source layout: `app/entry.kt` under
  it implements the platform handlers
  [platform-handler] of module `app.entry` in Kotlin, `app/entry.rs` does it
  in Rust. `salvo platform generate` writes them [cli-platform]. They are
  ordinary companion files [backend-companion] — discovered by the active
  backend's native extension, copied verbatim, gated on their module being
  reachable — with these differences.
  * The file is attributed to a module by its path **under the root**, and
    emitted as `platform/<module path>.<ext>`. Without that, a file's module
    would be its path from the source root, which no Salvo module is ever
    called, so it would never be reachable and never be copied.
  * Rust mounts a host file under `platform_<module>`; Kotlin gives it the
    package `salvo.platform.<module>`.
  * The module declaring a platform handler the program `use`s
    [platform-handler] or reaches a `platform fn` [platform-fn] **must** have an
    implementation file: the error
    names `salvo platform generate` rather than leaving the target toolchain
    to report a missing class against generated code [backend-never-wrong].
  * The tree is not the *customer's* alone: std ships its host classes
    under its own root too (`std/platform/fs/host.kt`), which is how a std
    `platform handler` is implemented. The embedded std is loaded with the
    active backend's extension, exactly as a source directory is.
  * Both backends' host files may coexist in one root, because discovery
    only ever picks up the active backend's extension — the same sources
    build for both targets.
* [stream-handle] **Every stream table draws its handles from one process-wide
  counter** (user decision 2026-09-29, 22; built the same day):
  `stream.fresh_handle()`, an `intrinsic` lowered to the runtime's atomic
  (`salvo_fresh_handle` / `SalvoStreams.freshHandle()`, in the host stream
  table's runtime file since 2026-10-03), used by `MemFs` and by
  `HostRawFs`. A handle handed to the wrong table is therefore *unknown* there
  — a trap [stream-provider] — and never another live stream, which is what
  per-table counters starting at 1 made likely. Making the mismatch a
  compile-time error is ROADMAP §4b item 4.
* [handler-state] **A handler's state initialiser may read its constructor's
  parameters** (user decision 2026-09-30): `handler Counting(start: Int) of
  Counter { at: Int = start * 2 }`. The state is built once, at construction,
  so the parameters are in scope read-only. Kotlin initialises the field from
  the constructor's `val`; Rust inside `new`.
* [host-splice] **Removed 2026-10-01** (user decision; the ABI decisions). Platform
  templates — `<m>.sv.kt` / `.sv.rs` files of host code with Salvo in
  `` `…` `` markers — made the reader keep two languages in mind at once and
  left the host's own tooling unusable. A platform handler or `platform fn`
  is implemented by an ordinary implementation file again ([platform-tree],
  [platform-fn]); the `*.sv.kt` / `*.sv.rs` names now belong to generated
  files the build never reads ([platform-root]). The markers, place
  ascription, the template grammars and the language server's template
  support went with them.
* [platform-abi] **The host ABI** (written down 2026-09-30, ROADMAP §4c step
  4): what hand-written or generated host code may rely on about the code the
  compiler emits — how a Salvo type is spelled in the target language, how a
  struct is built and read, how a union arm is made and tested, what `Checked`,
  `Reply` and a stream handle are, and what a platform handler's class must
  look like. Each backend states its half as `[rs-host-abi]` /
  `[kt-host-abi]`; anything *not* listed there (the scheduler's internals, the
  handle and monitor wrappers, generated helper names, overload suffixes) may
  change without notice.
  * The contract is **checked**, not just stated: `salvo platform generate`
    writes skeletons against it, std's own host files are compiled by every
    run that reaches them, and `the_aws_glue_compiles_against_both_sdks`
    compiles the generated aws glue against both SDKs whenever they are
    available locally — the case the contract exists for, a generator outside
    the compiler writing host code.
  * Changing an entry is a breaking change to every host file: update the
    backend spec, std's `platform/` files, the aws generator, and say so in
    COMPLETED.md.
  * **The host project** (ABI D2–D4, 2026-10-01): `compile`, `run`, `test`
    and `platform generate` write, into the project's own platform root for
    the backend, the files the root's implementation files compile against
    on their own — the **declarations** the platform surface reaches
    (`salvo_core::abi::platform_closure`: platform handlers' effects and
    parameters, platform fns' signatures, then struct fields, alias
    definitions, dot-named members, effect member signatures and
    prerequisites, to a fixed point), emitted as the build emits them but
    with no functions, handlers or qualifiers, one `<module path>.sv.<ext>`
    per declaring module (also written, empty of declarations, for a module
    that declares platform items, since its implementation file imports it);
    the runtime they need under `salvo/`; and the project files. Every file
    starts with a `GENERATED by salvo` header; one the program no longer
    produces is removed, and a file without the header is never touched.
    `analyze` and the language server write nothing. The build never reads
    them (a root's `*.sv.*` files are skipped), so each is generated to match
    the output's names, packages and derives exactly — `[kt-abi]`,
    `[rs-abi]`. Only the project's own modules count (std's too when the tree
    is std); a dependency checks in its own (D9), as std and aws do —
    `the_checked_in_host_projects_are_current_and_compile`.
* [platform-reply] **Host code can complete a `Reply` later, from any thread**
  (ROADMAP §4b item 2, 2026-09-29; the `aws` design's D3 — a service is a plain
  effect whose members take a `Reply` and return at once). A `platform handler`
  member may take a `Reply<T>` parameter — that was
  already legal, and renders as the runtime's reply type in both host
  signatures (verified, decision 7). What was missing was the runtime half:
  a reply sent from a host thread raced the scheduler, which saw a waiter with
  nothing queued and reported a deadlock before the answer arrived.
  * `reply.hosted()` — `SalvoReply::hosted` on Rust, `SalvoReply.hosted()` on
    Kotlin — hands the token to host code: it stops counting as an obligation
    the program owes [actor-on-idle], and the scheduler counts **one outside
    source of work** [threadsafe-platform] until it is sent, so the waiter is
    neither idle nor deadlocked. `send(value)` on the host reply delivers
    (enqueue only; the continuation runs as a later activation, never inside
    the call) and closes the source. The same mechanism a host thread's
    listener uses, made a host-facing surface.
  * **Exactly once.** A host reply dropped unsent is reported to the pool's
    fault sink on Rust (`Drop`) and the source is closed, so the waiter then
    reports its deadlock rather than hanging in silence; Kotlin cannot see a
    drop, so there the loss is undetected — **best-effort and documented**
    (user decision 2026-09-29, point 5). Kotlin refuses a second `send`.
  * The skeleton `salvo platform generate` writes states the contract above
    every member with a `Reply` parameter (`salvo_core::reply_contract_comment`).
  * Not in this slice: a host reply to a token **minted on another node**
    (refused with a message — it needs the typed wire path), and host
    *minting* of replies (decision 6, deferred with the writer pair).
* [platform-host-deps] **A project declares the target-language libraries its
  platform companions need in its manifest** (user decisions 2026-09-29, the
  `aws` module's first requirement of the language — ROADMAP §4b item 1; built
  the same day). `[rust] crates = { name = "1.0", other = { path = "…" } }`
  as Cargo spells a dependency; `[kotlin] artifacts = ["group:artifact:version"]`
  and `[kotlin] libs = "lib/kotlin"`, a directory of jars. Any platform handler
  over any library needs this; before it, `salvo run` called `rustc`/`kotlinc`
  bare and a companion could reach nothing the standard library did not ship.
  * **Merged across the build**: the project's declarations plus every
    dependency's [manifest-deps] (`Project::host_deps` → `HostDeps`), resolved
    once in `resolve_inputs` so a conflict is reported before anything is
    built. **The same crate or the same `group:artifact` at two versions is
    refused**, naming both parties — the build cannot settle it by picking
    (decision 4). A `path` in a crate spec is made absolute against the
    manifest that wrote it, since Cargo resolves it against the emitted file.
    Identical declarations deduplicate.
  * **Rust: Cargo only when crates are declared** (decision 2). With none the
    bare `rustc` path — and every checked-in example tree — is unchanged. With
    crates, the backend writes a `Cargo.toml` beside the crate root
    (`write_host_manifest`, called by `compile`, `run` and `test` after
    emission) and `program_command` runs `cargo build` into the same hidden
    `.salvo_bin` directory; `entry_hint` names cargo. A manifest the backend
    wrote earlier is removed when the crates go, recognised by its header —
    a hand-written one is never touched. See [rs-cargo].
  * **Kotlin: Gradle resolves the artifacts** (user decisions 2026-09-30,
    replacing decision 3's "not fetched"): with `artifacts` declared the
    backend writes a Gradle build beside the sources and runs it to resolve
    the classpath, as the Rust backend hands crates to Cargo; `kotlinc` still
    compiles. Every `*.jar` under the declared `libs` directories joins the
    classpath too, for jars no repository has. A `libs` directory that does
    not exist contributes nothing. See [kt-gradle] and [kt-classpath].
  * `clean_stale` skips hidden directories: `.salvo_bin` now holds cargo's own
    generated sources, which are not ours to delete.
  * **A dependency's libraries join only when its platform code is reached**
    (2026-09-29, the aws module's first customer): `dependencies_with_reached_platform`
    follows imports from the project's own files and includes a dependency
    whose reached modules have companions; everything else it declares stays
    out, so a program using only `aws.sqs`'s fakes builds with bare `rustc`
    and no jars. By imports, not by use, so it can over-include (a host module
    imported and unused) but never under-include. The project's own
    declarations always join.
  * Not done: a lock for host versions; repositories other than Maven
    Central; per-companion (as opposed to per-project) declarations.
* [host-tool] **Both backends' build tools are found the same way** (user
  decision 2026-09-30): `cargo` and `gradle` on PATH by default, and `[rust]
  cargo = "…"` / `[kotlin] gradle = "…"` to name another — a value with a path
  separator is a path relative to the manifest (a `gradlew` wrapper works as
  well as an installed Gradle, since they take the same command line), a bare
  name a PATH command. Only the **building** project's setting counts: which
  tool a machine runs is its call, never a dependency's. A tool is run only
  when its backend has host libraries to build; a missing one is reported
  naming the file and the key (`run_host_tool`). A tool's stdout goes to
  stderr — it builds the program and is not its output (a wrapper announces
  its first download on stdout).
* [effect-member-unique] Within one effect a member **signature** is
  unique: two members with the same name *and* the same parameter types are
  an error at the second declaration (source order, so the diagnostic is
  deterministic and fires once). Signatures are compared as *lowered* types,
  so two spellings of one type are the duplicate they are.
* [effect-dispatch] **A call is an effect-member dispatch exactly when the
  checker says so** (built 2026-10-06, `Checked::member_calls`): the call's
  span maps to the effect and to the member's index in its declaration order,
  and the instance it goes through is `effect_calls`. A backend never asks
  whether a name *could* be a member — a fn-typed local, or an ordinary fn
  sharing a member's name because no instance of the effect was in scope,
  is not one.
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
  * **Emitted names**: each overload is suffixed by its parameter types
    [fn-emit-name] (`close__InStream`, `close__OutStream` —
    `salvo_core::effect_member_name`, shared so the
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
  `platform handler`, which live in companions under the platform root
  [platform-tree]. A companion that collides with a
  generated file is an error.
* [backend-never-wrong] A backend must never emit silently wrong code:
  unsupported constructs are codegen/checker errors. Each backend spec
  lists its current deliberate cuts.

## The IR (`salvo-ir`; narrative in docs/language/The-IR.md)

* [ir-build] The IR builder turns the checked program into the IR, copying
  every answer the checker recorded. It infers nothing: where the checker
  left no answer, the node is `Unsupported` and an error names it, so a
  backend never renders a guess [backend-never-wrong]. Both backends read
  the IR only; the checker's span-keyed tables are the builder's inputs.
* [ir-nodes] The node set (`crates/salvo-ir/src/ir.rs`): every expression
  carries its resolved type; every reference is absolute (a `DeclId`, a
  local, or an interface member by index), so the IR has no imports;
  effects are leading parameters and a member call names its instance.
  Generic declarations stay generic, and a call carries its type arguments.
* [ir-types] Qualifiers are erased from every IR type except `Mut` (a backend
  may represent it as another type) and `proj` (a backend with ownership
  renders it as a borrow). A union arm keeps its qualifier tag only when
  erasure would merge two arms, a tagged `None` keeps its tag, and literal
  types collapse into their base [type-literal].
* [ir-coerce] A slot's coercion is explicit nodes around the value:
  `MakeUnion`, `Present`, `Rewrap`, `Widen`, `DropMut`.
* [ir-read] A read of a place carries `consume`: the value's life ends here.
  The IR marks consumption, never duplication; a backend with ownership
  copies a non-consuming read of a value it only borrows.
* [ir-op] `Op` is limited to scalar arithmetic and logic the language
  defines as primitive, and to the sign test of a `cmp` result. A comparison
  is a call to the resolved `cmp`/`eq`; on an intrinsic the backend
  reconstructs the native operator.
* [ir-branch] `Branch` is the subjectless choice: ordered conditions and an
  optional else. `if`/`elif`/`else` and a subjectless `when` lower to it.
* [ir-switch] `Switch` chooses on the arms of a subject union; each arm
  tests `Arm`, `Arms`, `None`, `Lit` or `Else`. `when x`, `?:`, `!` and
  `?.` lower to it.
* [ir-test] `Test` is `subject is …` used as a `Bool` condition.
* [ir-narrow] Narrowing is a `Narrow` statement: a new local at the narrower
  type, read from the subject's place, with a `Justification` naming the
  node that proved it (a test, a switch arm, a branch condition, a branch
  that left, a loop's totality, or a carried claim).
* [ir-alias] `Alias` names a place: every read and write of the local is
  one of the place [deduce-field].
* [ir-loop] There is one general loop, `Loop`, unconditional and left by
  `break`. `while c` is a loop whose body begins by breaking when `c` fails;
  `for` over an iterator is a loop that calls `next` and switches on the
  step. `ForEach` is the one loop with a subject, for the intrinsic
  containers a backend iterates natively [iter-for-native]. A loop's value,
  its `else` and its ran-flag are explicit locals, only when present.
* [ir-dump] The text form (`salvo_ir::dump`) is for reading and golden
  tests, never parsed back: one declaration per paragraph, `#n` node ids,
  `!read` for a consuming read, parameter modes before each parameter.

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
* [doc-module] **A module's documentation is the first comment run in its
  file, when that run documents no declaration** (user decision 2026-09-29,
  option (b)). Leading blank lines do not matter. The run is the module's
  when a blank line follows it — it then sits above nothing — *or* when the
  file's first token is `import` on the very next line: an import carries no
  docs, so a run directly above the first import cannot be anyone else's, and
  the header-then-imports layout needs no blank line. A run sitting directly
  above the first declaration is that declaration's [doc-comment], and the
  module then has no docs. Only the first run can qualify: a file opening
  with code has none. A file holding nothing but a comment documents itself.
  * Carried on the AST as `Module.docs: Vec<String>`, lines stripped as for
    [doc-comment]; the parser decides it before parsing the first item
    (`Parser::module_docs`), from the first token's line.
  * **A module's hover lists what it offers** (2026-10-03, user request):
    its first 20 exported functions, one signature per name, then "and N
    more", and its exported types by name; in its own file, everything.
  * **Shown wherever a module is named** [cli-lsp]: the module part of an
    `import` line — all of a whole-module import [mod-import-module], the
    segments before the item of a named one, *as one name* however many
    segments it has, so the hover covers `shop.shapes` whichever segment the
    cursor is on; an `@module` selector [fn-overload-at] [mod-suffix], which
    the checker records at the selector's span as `Checked::module_refs`
    mapped to the **full** path it resolved to (a selector that matched
    nothing or two modules is not recorded — it has a diagnostic); and a `by`
    site, whose hover appends a **Module** section for the module its comptime
    fn came from (`CompHover.module`). The hover's code line is `module
    <full path>`; below it the docs, with `[symbol]` references resolved
    against the module's top-level declarations [doc-symbol-ref]; then where
    it lives — the standard library, a dependency by name [manifest-deps], or
    the file. Go-to-definition on a module name lands at the top of its file.
  * Every std module with a header comment now carries it as module docs;
    the parser snapshots record them.

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
    program, which is what a redundant selector needs [fn-overload-at].
    `salvo analyze` counts warnings separately and exits 0 when there are no
    errors.
* [fn-ref-table] The checker records every fn-*name* reference —
  declaration names, call-site callees (incl. dot-notation), and
  fn-by-name uses — as `Checked::fn_refs: (file, name span) -> FnKey`.
  The LSP's hover renders the referenced declaration as a full
  source-like signature with an explicit return type (`None` when
  omitted) and the *effective* deduction clause in the `=>` spelling: the
  inferred/validated one (`Checked::deductions` [deduce-infer]) when
  available, else as declared. Consumed parameters render as `!p`; kept
  whole ones, and Copy scalars, are omitted; lends render as `proj(…)`;
  fn-type groups as `=>[f] …`. Effect-member calls
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
* [cli-analyze] `salvo analyze [--src DIR] [--format text|json]` (`--src` from the manifest when omitted [manifest])
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
* [cli-run] `salvo run [--backend NAME|*] [--src DIR] [--main FILE]
  [--target DIR] [--clean-target before|both]` (backend, src and main from the
  manifest when omitted [manifest]) compiles and then runs the
  program with the backend's own toolchain (user decisions 2026-09-05).
  One command from `.sv` source to program output.
  * `--backend` **defaults to `rust`** (user decision 2026-09-26), which is
    the default `salvo test` already had: its toolchain is the cheapest to
    start, and the two commands agreeing is worth more than making the choice
    explicit at every invocation. `compile` still defaults to `kotlin`.
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
* [lsp-completion] **`textDocument/completion` offers functions**
  (2026-10-03, user request): with no `.` before the word, every free fn the
  file can call; after `receiver.`, those whose first parameter's type name is
  the receiver's [fn-dot], then those whose first parameter is a type
  parameter, ranked after. The buffer is analysed with the word and its dot
  removed, so a half-written call parses; the receiver is the smallest typed
  expression ending at the dot. `.` is a trigger character. Since 2026-10-05:
  a struct receiver's **fields** rank first; **types** in scope are offered
  (ranked first where a type is written — after `:`, `->`, `<` or `|` — and
  after the fns elsewhere); on an `import` line, the next segment of every
  module path the written prefix extends and, once the prefix names a module,
  its exported items. Not yet: effect members, locals, keywords.
* [lsp-coalesce] **Edits are coalesced** (2026-10-05): a change notification
  only updates the overlay, and diagnostics are published once no message is
  waiting, so a completion asked for after several keystrokes is answered
  after one analysis rather than one per keystroke.
* [lsp-hover-iter-fn] An `iter fn`'s minter and generated `next` both hover as
  the `iter fn` written (`iter fn upto(n: Int) -> Emitted Int | Finished`),
  with its docs and without the `next` overload set.
* [lsp-effect-fix] A "no handler for effect `E` in scope" diagnostic offers a
  quick-fix adding `E` to the enclosing top-level fn's effect list (a new
  `[E]` after the parameters, `E` into `[]`, or `, E` at the end).
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
  * [lsp-std-source] **A declaration in the embedded std navigates to a
    read-only copy on disk** (2026-10-03, user request): the file is written
    under the system's temporary directory (`salvo-std-<version>/<name>`),
    rewritten when its content differs, and analysed as no project when
    opened. Doc links into std use the same copy.
  * A location's path is the file's name joined to the **analysis's**
    source root [manifest-discovery] — the project's `src` — not the
    workspace root (a defect until 2026-09-30: in a project whose `src` is
    a subdirectory, a definition and a hover's doc links named a path that
    did not exist).
  * Positions convert between byte offsets (Salvo spans) and UTF-16
    line/character pairs (the LSP default encoding).
* [cli-lang] `salvo lang tm-grammar [--out PATH]` emits the TextMate
  grammar consumed by the VS Code extension (`vscode/syntaxes/`); without
  `--out` it prints to stdout.
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
      `proj(list)`, the `hashed`/`ordered` claims of a `canbe` clause,
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
  `fs` is the case that prompted it.
  * Each entry renders from its own declaration (a member says which effect it
    belongs to, a fn shows its signature) with the module it comes from, which
    is the same information the scope ladder decides on [fn-overload-scope].
  * The index is built by the LSP's analysis pass while the resolver's scopes
    are alive and kept as owned data (`Analysis::overloads`), since a
    `Resolution` borrows the program. Only names with more than one
    declaration are stored.
* [cli-platform] `salvo platform generate [--backend NAME] [--src DIR |
  --main FILE]` writes the implementation skeletons into each backend's
  platform root [platform-root] — every backend the manifest builds, or the
  one named. `--src` and `--main` behave as in `salvo run` [cli-run].
  * **An implementation file** (`<m>.<ext>`) per module with platform
    handlers or `platform fn`s: a function with the real name for each
    platform fn, and a class (Kotlin) / struct with `new` (Rust) for each
    platform handler, implementing the effect's generated interface, every
    body stubbed `TODO` / `todo!`, with the [platform-reply] contract above a
    member taking a `Reply`. A platform fn's stub is the wrapper's own
    signature under the real name, so the two cannot disagree.
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
  * std's and a dependency's host files are **not** generated: they are
    shipped with them [platform-tree], so the command only ever writes into
    the customer's roots.
* [cli-ir] `salvo ir [--src DIR] [--module PATH] [--all]` prints the IR
  [ir-dump] of every user module, of one module, or (`--all`) of every
  module the program reaches, std included. It builds the IR exactly as the
  backends do, so it is the point to bisect when they disagree.
