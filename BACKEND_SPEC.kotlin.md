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

* [kt-package] Each module emits into its own Kotlin package
  `salvo.<module.path>` (`core.console` → `package salvo.core.console`),
  as `<module/path>.kt`. Cross-module name collisions are gone; the
  generated `unions.kt` lives in the root package `salvo`.
  * [mod-used-only] Only reachable modules that produce code are emitted;
    `unions.kt` is generated when any *emitted* file uses a wrapper size.
* [kt-imports] Files get generated Kotlin imports: a wildcard
  `import salvo.<module>.*` per foreign *emitted* module whose names the
  file uses (own module, template `imports:` lines, and
  `import salvo.*` for union wrappers round it out). An aliased Salvo
  import of a Kotlin-visible item (fn with body, struct, effect, handler)
  emits `import salvo.<module>.<name> as <alias>`, and call sites keep
  the alias; inlined externals, type aliases, and qualifiers need no
  alias import.
  * An aliased import of a *mangled* qualified overload
    ([kt-qual-mangling]) would map to the unmangled name — known gap in
    the same class as unchecked-context mangling.
* [kt-entry] `fn main() [use]` emits as `fun main()` with *no* effect
  parameters; the entry point is `salvo.<module>.MainKt` (`main.sv` →
  `salvo.main.MainKt`; the CLI prints it after compiling).
* [backend-companion] Companion `.kt` files next to a module's sources
  are copied verbatim into the output when the module is reachable. A
  companion must not collide with a generated file — its module should
  declare only `external` items (the LANGUAGE.md `complicated.kt`
  pattern), so it produces no code of its own.

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
* [type-array] `T[]` emits as `Array<T>` — always, including `Int[]` /
  `Double[]`. Decision (M7): no `IntArray`/`DoubleArray` specialization.
  Kotlin's specialized arrays are *unrelated types* to `Array<T>`, which
  would fracture generics, varargs/spread, and interop pass-through;
  boxing cost is accepted until profiling says otherwise (revisit with
  the Rust backend, where `T[]` maps to native arrays anyway).
* [kt-iter-iterable] `Iter<T>` maps to `Iterable<T>` (what Kotlin
  `for`-loops accept).
* [type-alias] Aliases expand structurally in the emitter too
  (`subst_ast_type`), matching the checker's expansion.

## Structs and variables

* [struct-decl] Structs emit as `data class` with `val` fields (`var`
  under `Mut`, defaults as `= expr`, optional nullable defaults `= null`).
* [struct-spread] `P {...p, f: v}` emits as `p.copy(f = v)`. The shallow
  copy aliases `Mut` fields where Rust deep-clones, which is
  unobservable because the checker consumes the spread base
  [deduce-consume].
* [kt-mutability] `let` emits `val`, or `var` when the name is assigned or
  `++`-incremented anywhere in the fn (mutation pre-scan);
  `Mut List<T>` emits `MutableList<T>` via the define's `Mut inline:`
  template [type-canbe-mut]; `Mut` struct fields emit `var`.
* [let-destructure] Tuple `let` uses native Kotlin destructuring; struct
  `let` lowers through a per-fn-unique `__destructuredN` temp.
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

## Control flow

* [while-value] [kt-loop-value] Kotlin loops are never expressions, so a
  value-position loop lowers to a `run {}` block: a `var __loopN` result
  local is assigned by the body's tail expression and by each
  `break value` (assign-then-`break`); with an `else`, a `__loopN_ran`
  flag guards an `if (!__loopN_ran)` block whose tail also assigns.
  * The local is a *nullable* temp initialized to `null`; when the
    checked join type has no `None` arm the block ends `__loopN!!`,
    otherwise plain `__loopN`. A `None`-typed or unchecked join uses
    `Any?` (pass-through, [type-unknown-lenient]).
  * `None`-typed tails/break values have no Kotlin payload: the
    expression stays a statement and the local is assigned `null`;
    `Nothing`-typed tails never fall through and stay statements.
  * Statement-position loops keep the plain Kotlin loop; an `else` needs
    only the ran-flag (no `run {}`); a `break value` whose loop value is
    discarded evaluates the operand for side effects only.
  * A block whose trailing expression is a loop routes it through the
    value lowering (`emit_value_block`), since the Kotlin block value
    would otherwise be the loop *statement* (`Unit`).
  * `break` routing uses an emitter stack of enclosing loop result
    locals, mirroring the checker's loop stack; `__loopN` ids are
    globally unique per file. Bare `return` inside a value-position loop
    in an iterator body is not re-targeted to `return@iterator` yet
    (shared limitation with all value blocks).

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
* [kt-nested-dot-name] A dot-named struct [name-dot] emits as a Kotlin
  **nested** class inside its namespace class — never `inner`, which
  would capture an outer instance and could not be constructed on its
  own. The namespace class grows a body holding its members; the member
  is declared under its own segment (`data class Id(...)`) while every
  *reference* keeps the dotted spelling (`Environment.Id`), which is
  valid Kotlin nested access and needs no import beyond the module
  wildcard [kt-imports].
  * Because the member is a plain nested class, the namespace struct must
    not be generic (enforced in `resolve`): a nested class cannot use the
    outer class's type parameters.
  * Dot-named *qualifiers* emit nothing (qualifiers erase
    [qual-erasure]); they only reach output through mangling, where the
    dot canonicalizes to the flat spelling
    (`label__EnvironmentTag`) [kt-qual-mangling].
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
* [kt-handler-template-return] A `define handler` member with a return
  type wraps its inline template in `return run { … }`: `run` yields the
  block's last expression, so both single-expression templates
  (`kotlin.random.Random.nextFloat()`) and statement-sequence templates
  work unchanged. Members without a return type keep their plain
  statement body.
* [kt-effect-params] Effect dependencies become leading function
  parameters named after the effect type (`Random<Int>` → `random_int`);
  `use` emits `val <name>: <EffectType> = Handler(...)`; effect member
  calls dispatch through the resolved handler expression
  (`random_int.next_random()`).
  * Generated effect-parameter and `use` variable names avoid the fn's
    parameters and locals (pre-scan of declared names; collisions get a
    numeric suffix: `console2`).
  * Handler resolution prefers the checker's effect tables
    (`use_effects`/`effect_calls`/`call_effects`) rendered through
    `kotlin_ty` — which must agree with `emit_type` on the same source
    type, since the result keys the effect-environment lookup. The
    string-keyed environment with base-name matching is the fallback for
    unchecked contexts (see PROGRESS.md "Emitter effect-environment
    fallback" under architectural facts).

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
* [internal-fn] Internal fns bypass define templates: the emitter lowers
  the call directly from the checker's resolved argument type. Kotlin
  implements `copy` [copy-fn] as [kt-copy]; any other internal fn is a
  codegen error.
* [kt-copy] `copy(x)` lowers type-directedly:
  * *identity* (emits just the argument) when the type is transitively
    immutable — scalars, `Str`, `None`, non-`Mut` lists of immutable
    elements, tuples/unions/fn values of immutable components, and
    struct types (any `canbe Mut` declaration included) whose value is
    not `Mut`-qualified and whose fields are transitively immutable
    [struct-mut]. Duplicating a reference to immutable data *is* a
    copy on the JVM.
  * `Mut List<E>` with immutable `E` → `.toMutableList()`;
  * a `Mut` struct whose fields are all transitively immutable →
    `.copy()` (the data class's shallow copy is exact there);
  * `T[]` with immutable `T` → `.copyOf()` (arrays are index-assignable
    without `Mut`);
  * anything else — nested mutability (`Mut List<Mut ...>`, a `Mut`
    struct with a `Mut`-typed field), generic `T`, `Iter`, unknown
    interop types — is a codegen error [backend-never-wrong].
  * Generic struct fields are checked under the instantiation's
    substitution; struct cycles are assumed immutable along the
    visiting spine.
* [readonly-return] Derived returns erase: the result already is the
  alias on the JVM, and the checker's caller-side links keep it aligned
  with Rust's borrows.
* [fn-contract] Contracts erase (Kotlin lambdas alias; the checker's
  contract application keeps the semantics aligned with Rust's modes);
  a named fn passed by value emits the function-reference syntax
  (`::name`).
* [once-fn] `Once` erases: the parameter emits the ordinary Kotlin
  function type, and the at-most-once protocol is enforced by the
  checker alone.
* [linear-discard] `discard(x)` lowers to `(x).let {}` — evaluate and
  ignore [internal-fn]; linearity is purely static [linear-static], with
  no runtime component on the JVM.
* [fate-lambda] Kotlin lambdas capture lexically (aliases), unchanged
  by L4: the checker's capture contract (mutated captures consumed at
  creation, closures poisoned by root mutations) is what keeps the
  alias semantics aligned with the Rust backend's borrow-captures.
* [fate-move-mode] Kotlin emission is *unchanged* by binding modes:
  bindings alias on the JVM in every mode. Parity with the Rust
  backend's real moves comes from the checker — a move-mode binding
  consumes its ancestors, so no program can observe alias-vs-move —
  and the same holds for tracked moved-position projections of mutable
  data.

## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* multi-spread struct literals;
* early `return` inside expression-position lambdas;
* tuples beyond `Pair`/`Triple`;
* struct literal without an inferable type;
* `copy` of a type with nested mutability or an unknown/generic type
  [kt-copy].
