# Salvo Kotlin Backend Spec — Labeled Rules

The Kotlin interpretation of [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md). Load
this file (only) when working on the Kotlin backend
(`crates/salvo-backend-kotlin`, `std/**/*.kotlin.sv`).

Conventions:

* Rules repeated from LANGUAGE_SPEC.md keep their label; the sub-bullets
  here are the *Kotlin-specific* decisions, additive to the core rule.
* New Kotlin-only rules are prefixed `kt-`. Like all rule labels, they are
  referenced from code and tests (`grep -rn '\[kt-qual-mangling\]'`).

## Output layout

* [kt-package] All modules emit into the single Kotlin package `salvo`, as
  `<module/path>.kt` (name collisions across modules are possible;
  per-module packages + generated imports are M7).
  * `unions.kt` is generated whenever any wrapper size is used
    ([kt-union-wrappers]); a std module is emitted only if it produces
    code (currently just `core/console.kt`).
* [kt-entry] `fn main() [use]` emits as `fun main()` with *no* effect
  parameters; the entry point is `salvo.MainKt`.

## Type mappings

* [type-basic] Internal types map natively: `Str`→`String`,
  `Bool`→`Boolean`, `Byte`/`Int`/`Long`/`Float`/`Double`/`Char` keep
  their names, `Any`→`Any`, `Nothing`→`Nothing` (`emit_named_parts`).
* [kt-none-unit] `None` as a return type is `Unit`; `None` as a union arm
  is nullability ([kt-union-nullable]).
* [type-str] Interpolation emits native Kotlin templates, using the short
  `$name` form when the interpolated expression is a bare identifier.
* [type-nullable] `T?` maps to Kotlin nullability: `is None` → `== null`,
  `x!` → `!!`.
* [type-tuple] Tuples of size 2/3 map to `Pair`/`Triple`; larger tuples
  are a codegen error ([backend-never-wrong]).
* [type-array] `T[]` emits as `Array<T>`; `IntArray`/`DoubleArray`
  specializations are future work (M7).
* [kt-iter-iterable] `Iter<T>` maps to `Iterable<T>` (what Kotlin
  `for`-loops accept).
* [type-alias] Aliases expand structurally in the emitter too
  (`subst_ast_type`), matching the checker's expansion.

## Structs and variables

* [struct-decl] Structs emit as `data class` with `val` fields (`var`
  under `Mut`, defaults as `= expr`, optional nullable defaults `= null`).
* [struct-spread] `P {...p, f: v}` emits as `p.copy(f = v)`.
* [kt-mutability] `let` emits `val`, or `var` when the name is assigned or
  `++`-incremented anywhere in the fn (mutation pre-scan);
  `Mut List<T>` emits `MutableList<T>`; `Mut` struct fields emit `var`.
* [let-destructure] Tuple `let` uses native Kotlin destructuring; struct
  `let` lowers through a `__destructured` temp (uniquing two such `let`s
  in one block is a known M7 leftover).
* [is-binding] `is T name` bindings emit `val name = subj as T` at the top
  of the matched branch (relies on subject purity); `while x is T name`
  re-declares the binding per iteration at the top of the loop body.

## Unions

* [kt-union-wrappers] Wrapper unions emit as generated sealed hierarchies
  in `unions.kt`: `sealed interface UnionN<out T1..TN> { val value: Any? }`
  with `data class UN_i(override val value: Ti)` per arm, generated for
  every size the program uses.
* [union-arm-identity] Arm indices from the checker map 1:1 onto the
  `UN_i` wrapper variants (positional over the declared type's non-`None`
  arms, qualifiers erased).
  * Wrap at boundaries: `U2_1<Int, String>(expr)`; re-wrap between union
    reprs via a `let { when (it) { is U3_2<*,*,*> -> U2_1<...>(...) } }`
    chain (unmatched source arms are unreachable at runtime).
  * Narrowed ident uses unwrap in place: `(x.value as Int)`. The cast is
    *required* even when kotlinc smart-casts: narrowing may come from
    `elif` exclusion (no Kotlin smart cast), and `value` is typed `Any?`
    on the sealed interface.
  * `is` lowering: single arm → `x is U3_2<*, *, *>` (star projections
    required — kotlinc rejects bare generic classes in `is`), multi-arm →
    `||` chain, all arms / `is None` → null tests. Interpolating a
    still-union value appends `.value`.
* [kt-union-nullable] A `None` arm becomes outer nullability
  (`Union2<..>?`); exactly one non-`None` arm stays plain `T?` (no
  wrapper).
  * A `T?`-subject `when` lowers to a subject-less Kotlin `when` whose
    last branch becomes `else` (kotlinc demands one on expression `when`;
    sound because the checker proved exhaustiveness).
* [when-union-subject] `when` over a wrapper union lowers to Kotlin
  `when (subj)` over the sealed wrappers; kotlinc re-proves the
  exhaustiveness the checker established ([when-exhaustive]).

## Qualifiers

* [qual-erasure] Qualifiers erase entirely from emitted Kotlin; wrapper
  arm choice, casts, predicate calls, and mangled names are what survive.
* [kt-qual-mangling] Overloads identical after erasure get a deterministic
  `__Qual` suffix on the qualified overload (`full_name__Surname`),
  applied consistently at declarations and checker-resolved call sites.
  * The collision test compares *emitted* Kotlin parameter strings, so
    the `Mut List` → `MutableList` mapping naturally avoids false
    collisions. Unchecked (arity-fallback) calls to a mangled overload
    would emit the base name — known leftover.
* [is-qualifies] Each predicate qualifier's `qualifies` fn emits as a
  top-level `fun Q_qualifies(...)`; a predicate `is` check becomes a call
  (multiple qualifiers `&&`-chain).
* [is-qualifies-effects] `qualifies` effects are threaded as leading
  handler arguments of the `Q_qualifies` call.
* [qual-field-override] Field-override accesses emit a cast + assert:
  `(person.surname as String)`.

## Effects

* [effect-decl] Effects emit as Kotlin `interface`s.
* [kt-handler-class] All handlers — external ones included — emit as
  Kotlin *classes* (never `object`s) and are instantiated at their `use`
  site; state fields become `private var`, constructor params
  `private val`.
* [kt-effect-params] Effect dependencies become leading function
  parameters named after the effect type (`Random<Int>` → `random_int`);
  `use` emits `val <name>: <EffectType> = Handler(...)`; effect member
  calls dispatch through the resolved handler expression
  (`random_int.next_random()`).
  * Handler resolution prefers the checker's effect tables
    (`use_effects`/`effect_calls`/`call_effects`) rendered through
    `kotlin_ty` — which must agree with `emit_type` on the same source
    type, since the result keys the effect-environment lookup. The
    string-keyed environment with base-name matching is the fallback for
    unchecked contexts (see PROGRESS.md "Emitter fallback mechanism").

## Functions

* [fn-variadic] `...xs: T[]` emits `vararg xs: T`; a spread argument
  `...xs` emits `*xs`.
* [fn-iterator] Iterator fns emit
  `return Iterable<T> { iterator { ... } }`; `yield x` → `yield(x)`; bare
  `return` → `return@iterator`.
* [fn-dot] Dot-notation calls that resolve to a known fn/define/effect
  member are normalized to `f(base, args)`; unknown methods stay Kotlin
  method calls (`base.f(args)`) for interop ([type-unknown-lenient]).
* [backend-define-inline] Define templates expand inline at call sites;
  `imports:` lines are hoisted per generated file
  ([backend-define-imports]).

## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* loop-as-value, `break value`, loop `else` (M6);
* multi-spread struct literals;
* early `return` inside expression-position lambdas;
* tuples beyond `Pair`/`Triple`;
* struct literal without an inferable type.
