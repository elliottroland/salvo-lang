# Salvo Rust Backend Spec — Labeled Rules

The Rust interpretation of [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md). Load this
file (only) when working on the Rust backend
(`crates/salvo-backend-rust`, `std/**/*.rust.sv`).

Conventions:

* Rules repeated from LANGUAGE_SPEC.md keep their label; the sub-bullets
  here are the *Rust-specific* decisions, additive to the core rule.
* New Rust-only rules are prefixed `rs-`. Like all rule labels, they are
  referenced from code and tests (`grep -rn '\[rs-borrows\]'`).
* rustc is the safety net: generated code that violates these rules fails
  to *compile*, never silently misbehaves at runtime
  ([backend-never-wrong]).

## Output layout

* [rs-crate] The output is a single-binary Rust crate compiled straight
  from the emitted files (`rustc --edition 2021 main.rs`). The module
  declaring `fn main` becomes the crate root; it starts with the crate
  attributes (`#![allow(...)]` for cosmetic lints the generator does not
  fight: `non_snake_case`, `unused_parens`, `unused_mut`, ...) and one
  `#[path = "..."] mod <mangled>;` declaration per other emitted file.
  For a library compile (no `main`), a synthetic `lib.rs` carries the
  attributes and mod declarations.
  * A Salvo module `core.console` emits to `core/console.rs` and mounts
    as `mod core_console` (path parts joined with `_`): Rust module
    paths are flat, generated imports use `crate::core_console::*`.
  * The generated union enums live in `unions.rs`, mounted as
    `mod unions` [rs-union-enums].
* [rs-imports] Files get generated `use` items: `use crate::<mod>::*;`
  per foreign *emitted* module whose names the file uses,
  `use crate::unions::*;` when the file touches union wrappers, plus
  template `imports:` lines ([backend-define-imports]). An aliased Salvo
  import of a Rust-visible item emits
  `use crate::<mod>::<name> as <alias>;` and call sites keep the alias.
  * Items referenced through glob imports must be `pub`: every emitted
    item (fn, struct, trait, impl fn, enum) is `pub`, struct fields
    included.
* [rs-entry] `fn main() [use]` emits as Rust `fn main()` with no effect
  parameters. The CLI reports the crate-root file as the entry point.
* [backend-companion] Companion `.rs` files are copied verbatim and
  mounted like generated modules. A companion must not collide with a
  generated file.

## Type mappings

* [type-basic] Internal types map natively: `Str`→`String`, `Int`→`i32`,
  `Long`→`i64`, `Float`→`f32`, `Double`→`f64`, `Bool`→`bool`,
  `Char`→`char`, `Byte`→`u8`, `Nothing`→`!` (LANGUAGE.md's original
  `u64` for `Long` was a spec bug — `Long` is signed; fixed during M8).
  `Any` has no Rust mapping yet: referencing it is a codegen error
  ([backend-never-wrong]).
* [kt-none-unit]-equivalent: `None` as a return type is `()` (omitted);
  `None` as a union arm is `Option` [rs-option].
* [rs-option] `T?` maps to `Option<T>`: `None`→`None`, `is None`→
  `.is_none()`, `x!`→`.unwrap()`. Optionals are *physical* in Rust, so
  the checker's `WrapOption` coercion emits `Some(code)` [type-nullable]
  (Kotlin ignores the same coercion).
* [type-array] `T[]` maps to `Vec<T>`; literals emit `vec![...]`;
  `Int[5] { i: Int -> 0 }` emits an iterator-map-collect; indexing casts
  the `i32` index (`v[(i) as usize]`).
* [rs-iter-vec] `Iter<T>` maps to `Vec<T>`, and iterator functions
  (`yield`) are *eager*: the body collects into a `__yielded: Vec<T>`
  local (`yield x` → `__yielded.push(x)`, bare `return` →
  `return __yielded`, falling off the end returns it too)
  [fn-iterator]. Deliberate cut: Rust generators are unstable; eager
  collection changes side-effect *timing* (not values) versus Kotlin's
  lazy sequences, and an infinite iterator would not terminate.
* [type-tuple] Tuples map to native Rust tuples (any size).
* [rs-tuple-index] A tuple index ([expr-tuple-index]) is Rust's own
  positional field: `t.0` emits as `t.0`, nesting included (`t.1.0`).
  Reads clone like any other projection in owned position, and a narrowed
  element unwraps as a field does ([flow-place] [rs-option]).
* [type-str] Strings are `String` (owned). Plain string literals emit
  `"...".to_string()`; interpolation emits `format!("{}...", args)`.
* [name-dot] Dot-names *flatten*: `Environment.Id` emits as
  `EnvironmentId`, in declarations, references and mangled overload names
  alike (`rs_ident` is the single funnel, and a dot cannot reach it from
  any other kind of Salvo name — dots are invalid in Rust identifiers).
  * A nested `pub mod Environment` was rejected: Rust puts modules and
    structs in one *type namespace*, so it collides with
    `pub struct Environment` (E0428 — "`Environment` must be defined only
    once in the type namespace of this module"), and the namespace struct
    is required to exist. Flattening is what makes the language-level
    collision ban load-bearing [name-dot].
* [type-alias] Aliases expand structurally in the emitter (same
  `subst_ast_type` approach as Kotlin).
* [qual-erasure] Qualifiers erase from emitted types; what survives is
  arm choice, casts, predicate calls, mangled names — and the borrow
  modes that `Mut` implies [rs-borrows].
* [type-canbe-mut] Rust maps `Mut T` to the *same* type as `T` (no
  `Mut inline:` in the std rust defines): mutability is expressed in
  bindings and references (`let mut`, `&mut`) [rs-borrows], not in the
  type. Struct `Mut` works the same way (all struct fields are plain
  fields; assignability is enforced by the checker).

## Ownership and borrowing [rs-borrows]

The central design (per LANGUAGE.md "Deductions"): the checker's deduction
tables (`Checked::deductions`, [deduce-syntax] [deduce-infer]) are the
ownership contract. Salvo source has no references; the Rust backend
derives them mechanically:

* **Parameter modes.** For each parameter of a fn with a deduction entry:
  * *omitted* from the deductions (`kept == false`) → the parameter is
    **moved**: it is passed **by value** (`T`). The Salvo checker
    guarantees the caller no longer uses the argument, so the move is
    always legal.
  * *kept* and its declared type carries `Mut` → **`&mut T`** (the callee
    may mutate in place; the caller observes the mutations).
  * *kept* without `Mut` → **`&T`** (shared borrow).
  * **Copy exception:** parameters of Copy scalar types (`Int`, `Long`,
    `Float`, `Double`, `Bool`, `Char`, `Byte`) are always passed by value
    — a borrow would be noise, and copying preserves the semantics of
    both kept and moved deductions.
  * Variadic parameters are always owned `Vec<T>` (the caller assembles a
    fresh vector at the call site) [fn-variadic].
  * Fns outside the deduction tables (qualifier `qualifies` fns, handler
    members) default to the *kept* rule for every parameter: `&T` /
    `&mut T` per `Mut`, scalars by value. Effect member fns use the same
    rule [rs-effects].
* **Argument rendering.** Call sites consult the resolved callee's
  parameter modes:
  * moved position → the owned rendering of the argument (see below);
  * `&T` position → `&arg` for place expressions; a bare identifier that
    is already a reference binding passes as-is (Rust auto-reborrows and
    `&mut T` coerces to `&T`);
  * `&mut T` position → `&mut arg` for places; `&mut` parameter bindings
    pass as-is (implicit reborrow).
* **Owned rendering.** Expressions are emitted *owned* by default:
  * identifiers bound by reference (`&T`/`&mut T` parameters) clone
    (`x.clone()`); owned locals move (`x`); Copy scalars copy;
  * field reads and index reads of non-Copy types clone
    (`person.name.clone()`, `v[i as usize].clone()`) — moving out of a
    place behind a (possible) reference is not generally legal, and the
    checker does not track last-use. Exception [fate-move-mode]: a
    projection in `Checked::moved_projections` (a moved-position
    projection of mutable data whose roots the checker consumed)
    renders as the raw place — a real partial move;
  * call results, literals, and constructed values are already owned.
  * Assignment targets, `is` subjects, and borrow positions use the raw
    place (no clone).
  * **Borrow-mode bindings borrow [rs-borrow-locals] (S3):** a `let`
    from a *pure place* — a bare identifier or field/index chain with
    no coercion, narrowing unwrap, or field cast — whose bind event is
    not move-mode and whose name is never reassigned in the fn emits a
    real borrow: the local holds `&T` (`let mut n = &person.name;`,
    bare pass-through for an already-`&` root, `&*x` reborrow for
    `&mut` roots) and registers as a reference binding, so reads thread
    through the existing rendering (clone in owned positions, bare in
    borrow positions, `*x` for Copy). Borrow-mode `for` loops over
    pure-place iterables with plain ident bindings and concrete
    *non-union* element types iterate *by reference* (`for x in &xs`,
    or bare for an already-borrowed parameter) with the loop variable
    as a reference binding — no collection clone. Everything else
    (mixed joins, reassigned names, union/optional elements,
    value-position loops, `is`/`when` bindings, coerced values) keeps
    the fate-link clone — sound by restriction, since links union
    across branches and poison covers every observation (decision S3a,
    2026-09-02: mixed joins are an emission fallback, not a semantic
    restriction — no program's legality changes). Checker legality
    aligns with NLL because a borrow's last use precedes any root
    mutation/move in checker-legal code; the known loud exception is a
    single call that both passes a borrow-emitted local and moves its
    root (rustc E0505, checker-legal by left-to-right ordering).
  * **Lambdas emit plain (borrowing) closures [fate-lambda]:** captures
    are rustc borrow-captures, which alias — the same semantics as
    Kotlin's lexical capture, so parity is direct. Checker-legal
    programs pass borrowck because a closure is poisoned by a root
    mutation (its borrows end before the mutation under NLL) and
    mutated captures are consumed at creation (no later conflicting
    use). Known loud leftover: *returning or storing* a
    capture-carrying closure is a rustc lifetime error the checker does
    not reject; the recorded refinement is `move`-closure emission with
    hoisted clones (`Checked::lambda_captures` carries the capture
    list), which needs a treatment for captured effect-handler locals
    first.
  * **Move-mode bindings move [fate-move-mode]:** a bind event in
    `Checked::binding_modes` (the checker consumed the ancestors at the
    binding) emits the value as its raw place — a real move, partial
    for projections (`let name = person.name;`) — and a `for` loop
    whose iterable span is in the table iterates *by value* (the
    collection moves into the loop). This is what makes inferred
    consuming pipelines zero-clone end to end. `is`/`when` move-mode
    bindings still clone (restriction-valid: the checker consumed the
    subject, so the difference is unobservable).
* **Local bindings.** Every `let` emits `let mut` (the crate-root
  `#![allow(unused_mut)]` silences the cosmetic lint): Salvo mutability
  (assignment, `++`, `Mut` methods, `&mut` argument positions) is
  otherwise undecidable locally. Reassigned parameters get a `mut`
  binder.
* **Lifetimes.** Struct fields are owned and — with one deliberate
  exception — results are owned, so functions are
  lifetime-elision-friendly. The exception [readonly-return]: a
  derived-return fn returns `&T` / `Option<&T>`; elision covers the
  single-reference-parameter case, and with more reference parameters
  a `'a` is generated mechanically onto the annotated parameter and
  the return. Return values render as borrows (`Some(&place)`, bare
  for already-`&` bindings, pass-through for forwarded derived
  calls); std's `first` define is `${list}.first()` — clone-free.
* **Generic bounds.** Every generic parameter gets a `Clone` bound
  (`<T: Clone>`) — the owned-rendering rule may clone values of generic
  type. Structs additionally `#[derive(Clone, Debug)]`.
* Deliberate simplicity, accepted costs: kept-parameter arguments are
  never moved even when it would be their last use (a clone happens
  instead); rustc's borrow checker remains the final authority — a
  program that emits but does not borrow-check is a compiler bug, not a
  user error.

## Unions [rs-union-enums]

* [kt-union-wrappers]-equivalent: wrapper unions emit as generated
  enums in `unions.rs`:
  `pub enum UnionN<T1..TN> { U1(T1), .., UN(TN) }` with
  `#[derive(Clone, Debug)]`, per-arm accessor methods
  (`pub fn u1(&self) -> &T1`, panicking on the wrong arm — unreachable
  when the checker's tables are right), and a `Display` impl (bounded on
  every arm being `Display`) so still-union values interpolate directly.
* [union-arm-identity] Arm indices from the checker map 1:1 onto the
  `Ui` variants (positional over the declared type's non-`None` arms,
  qualifiers erased).
  * Wrap at boundaries: `UnionN::<A, .., Z>::Ui(code)` (turbofish —
    the other type parameters are not inferable from one arm), wrapped
    in `Some(...)` when the target union has a `None` arm.
  * Narrowed place reads unwrap in place — `x.u2().clone()` for a
    variable, `h.result.u1().clone()` for a field [flow-place]
    (`….as_ref().unwrap().u2().clone()` for a nullable repr). Unlike
    Kotlin, Rust also unwraps a `T?` repr narrowed to its value arm:
    `x.unwrap()` for Copy scalars, `x.as_ref().unwrap().clone()`
    otherwise — there is no smart cast to lean on. The result is an owned
    temporary, so a narrowed place is *not* a pure place
    ([rs-borrow-locals]): it cannot be borrowed directly. `is` tests, `is`
    bindings and `match` subjects read the storage instead.
  * `is` lowering ([is-narrowing], `is_tests` table): single arm →
    `matches!(subj, UnionN::Ui(_))`; multi-arm → a `|` pattern; all
    arms → `subj.is_some()` when nullable, `true` otherwise;
    `is None` → `subj.is_none()`; nullable wrappers test
    `Some(UnionN::Ui(_))` patterns.
  * Re-wraps between reprs ([let-infer]): a `match` mapping arms by type
    equality, `unreachable!()` for unmatched source arms, `None => None`
    when both sides are nullable.
* [when-union-subject] `when` lowers to a Rust `match` on the subject
  place (`match subj`): variant patterns bind nothing, so the match
  borrows rather than moves; branch checks become variant patterns (or
  `Some(..)`/`None` for nullable subjects); rustc re-proves the
  exhaustiveness the checker established [when-exhaustive]. `match` is
  an expression, so `when`-as-value needs no extra lowering.

## Control flow

* [while-value] [rs-loop-value] Rust `while`/`for` are statements, so a
  value-position loop lowers to a plain block expression with a
  `let mut __loopN: Option<T> = None;` result local assigned by the
  body's tail and by `break value`s (assign-then-`break`); an `else`
  uses a `__loopN_ran` flag. When the checked join type has no `None`
  arm the block ends `__loopN.unwrap()`; when the join is itself
  optional the local *is* the join type and tails assign it directly
  (their `WrapOption`/wrap coercions are already recorded by the
  checker).
* [if-else-none] `if` is an expression in both languages; a missing `else`
  on a value-position `if` emits `else { None }` (the branch values carry
  `WrapOption` coercions). Statement-position branches emit their tails as
  statements.
* [when-condition] [rs-when-cond] Rust has no subject-less `match`, so a
  subject-less `when` lowers to the `if`/`else if`/`else` chain it is —
  the emitter reuses `if`'s statement and value paths. The mandatory
  `else` makes the chain total, so nothing needs the `else { None }` filler
  of [if-else-none] and no `unreachable!()` arm is generated. (A
  `match () { () if cond => … }` would also work and was rejected: it adds
  a scrutinee that means nothing and reads worse than the chain the source
  already is.)
* [loop-while-is] `while x is T (name)?` re-tests in the loop condition
  and re-binds per iteration at the top of the body (same shape as
  Kotlin).
* [rs-postincrement] Rust has no `++`: statement-position `i++` emits
  `i += 1;`; value-position emits `({ let __t = i; i += 1; __t })`.
* [qual-widen] [rs-widen-shadow] A `^` check emits the same test `is` would
  (or `true` when the qualifiers are statically present — qualifiers are
  erased, so widening is a typing act). Where it *peels a wrapper arm*, the
  widened value is bound to a **shadowing local** at the top of the branch
  (`let mut nested = nested.u1().clone();`), so reads of the subject and any
  nested `when` see the inner value. The binding kind is saved and restored
  around the branch, since the shadow is owned where the outer binding may be
  a borrow.
  * Without the shadow the emitted code compiles and is **wrong**: the nested
    `match` scrutinizes the outer wrapper, whose arm 0 is the one the outer
    test already took, so the second inner branch becomes dead code. Caught
    by running the feature's own demo (`Display` on the generated union had
    been masking it in the printed output).
  * `^` on a *projection* (`p.result ^ Ok`) is a reported codegen error for
    now: the materialization needs a plain variable to shadow.
* [fn-effects] [rs-fn-effect-params] A fn type's effects are **leading
  `&mut dyn Effect` parameters** of the closure: `(s: Str) [Logger] -> Str`
  renders as `&mut impl FnMut(&mut dyn Logger, &String) -> String`, a lambda
  as `|logger: &mut dyn Logger, s| …`, and the call passes the instance
  first. Nothing is captured, which is what lifts the fusion cut above — and
  a fused value coerces into the `&mut dyn` parameter, so this is
  fusion-agnostic (verified by hand before implementation).
  * Inside a lambda body an effect resolves to the lambda's *own* parameter:
    the effect environment is searched innermost-first, so a parameter
    shadows the enclosing fn's value rather than capturing it.
  * A **named fn** passed by value gets an adapter closure taking the
    *expected* effects and forwarding the ones it declares — a pure fn is
    handed them and ignores them (the variance rule). A mismatch the
    checker's fits rule should have caught is an internal-error codegen
    diagnostic, never a guess [backend-never-wrong].
* [defer] [rs-defer-splice] Rust has no `finally`, and a `Drop` guard
  cannot be used: `close(f)` consumes the handle, so the guard would have
  to own `f` from the `defer` onward, making it unusable for the rest of
  the block (a `&mut` capture trades that for `E0499` at the next use).
  So the body is **spliced** — emitted at every exit of its block — which
  is [defer]'s meaning literally, and leaves no runtime construct behind.
  * The body is rendered **once**, at the `defer` statement (in the scope
    it was written in, with that point's effect environment and bindings),
    and re-indented at each splice site. Sites: the end of the block
    (`emit_block_stmts`, skipped when the block's last statement already
    exits — the splice would be dead code rustc still borrow-checks), each
    `return` (all deferred blocks of the fn, `defer_floor` stopping at a
    closure boundary), and each `break`/`continue` (those registered
    inside the loop, tracked by `loop_defer_floors`).
  * A value given away at the exit is computed first, so `return v` with
    deferred code behind it becomes
    `let __deferred_valueN = v; <deferred>; return __deferred_valueN;` —
    and a value-position block hoists its tail the same way
    (`emit_value_block`), since the deferred statements would otherwise
    become the block's value.
  * **Known divergence** from Kotlin's `finally` lowering (accepted, user
    decision 2026-09-04): a panic out of a std define unwinds *past* the
    splice, so deferred code does not run on a crash path, where the JVM's
    `finally` would run it. See [kt-defer-finally].
* [abort] [rs-abort-controlflow] A fn declaring `[Abort<M>]` returns
  `ControlFlow<M, T>` — the message type *is* `ControlFlow`'s `Break`
  payload, so the propagation falls out of the design rather than being
  imposed on it. `Abort` is filtered out of the effect *parameters* (it is a
  return shape, not a capability), and the effect declaration itself emits
  no trait: it has no handlers to implement.
  * `abort(m)` is `return ControlFlow::Break(m)` — no handler, no dispatch,
    no allocation. `return v` becomes `ControlFlow::Continue(v)`, and a
    `None`-returning fn ends with `return ControlFlow::Continue(())`.
  * A call that may abort unwraps with `?` — but only when the abort would
    leave *this* fn unchanged. Inside a `try`, when the message must be
    wrapped into a union arm, or when deferred blocks have to run first, the
    propagation is written out as `match call { Continue(__v) => __v,
    Break(__m) => <transfer> }`. That is an *expression*, so it works in
    argument position with no hoisting — and it is the only way the deferred
    release can run on the abort path, since `?` returns without it.
  * Verified by hand before implementation (`?` on `ControlFlow` is stable;
    a may-abort call in a loop stays a loop, no trampoline): the three
    compositions the roadmap asked for are in the E3 notes.
* [try] [rs-try-label] `try { ... }` is a **labelled block**
  (`'try_N: { ... }`), not a closure. The sketch's closure
  (`(|| -> ControlFlow<..> { .. })()`) would have to capture the fn's
  effect parameters and any local the body mutates — the same exclusivity
  trap the fusion hits; a labelled block captures nothing.
  * The price is that `?` cannot be used inside a `try` body (it would
    return from the *fn*), so every may-abort call there takes the `match`
    form above and `break`s the label with the outcome's aborted arm. The
    body's tail is wrapped into the `Ok` arm (arm 0); a body that always
    leaves still needs a value for the block, which the checker made
    `Ok None` [try].
  * Nesting needs no token: the label decides where a `break` lands, so an
    inner delimiter cannot swallow an outer abort [try-innermost].

## Qualifiers

* [is-qualifies] Each predicate qualifier's `qualifies` fn emits as a
  top-level `pub fn Q_qualifies(...) -> bool`; a predicate `is` check
  becomes a call (multiple qualifiers `&&`-chain). Parameters follow the
  default kept rule [rs-borrows].
* [qual-field-override] Field-override accesses read the value out of
  the declared representation: `person.surname.as_ref().unwrap().clone()`
  for a `T?` field narrowed to `T` (the cast-and-assert of
  [qual-field-override]; a wrong override panics on `unwrap`).
* [rs-fn-mangling] Rust has no overloading: when several fns *with
  bodies* share a name, the qualified-overload suffix rule of
  [kt-qual-mangling] applies first (`full_name__Surname`), and any
  overloads still colliding after erasure get a deterministic positional
  suffix (`name__2`, `name__3`, ... in declaration order; the first
  keeps the base name). Call sites resolved by the checker use the same
  mangled name; unchecked arity-fallback calls share Kotlin's known
  mangling gap.

## Effects [rs-effects]

* [effect-decl] Effects emit as Rust `pub trait`s whose methods take
  `&mut self` (handlers are stateful). The abort effect is the exception:
  it emits nothing, since it has no handlers [rs-abort-controlflow].
* [effect-args-hoisted] An argument whose code reaches an effect value the
  *same call* threads is hoisted into a `let` before the call
  (`{ let __a1 = inner(&mut console, 1); outer(&mut console, __a1) }`).
  Without it, two effectful calls in one expression borrow the same
  `&mut dyn` parameter twice (`E0499`) — a shape Kotlin accepts and Rust
  rejects, so it was a live parity divergence (found and fixed 2026-09-04
  while building [try], whose delimiter reads naturally as a call
  argument). The hoist predicate is per-call: an argument mentioning a
  *different* effect value is left alone, so output churn is limited to the
  shapes that would not compile.
* [effect-handler] Handlers emit as `pub struct H<..> { ctor-params,
  state }` + `impl H { pub fn new(ctor-params) -> Self }` (state fields
  initialized from their declared defaults) + `impl Effect for H`.
  Handler member bodies access ctor params and state through `self.`.
  External handlers inline their `define handler` templates as method
  bodies, same shape. A ctor param that is a *dependency* is neither a
  field nor a `new` parameter, and the trait impl is replaced by a
  generated one — see [rs-effect-fusion].
* [kt-effect-params]-equivalent: effect dependencies become leading
  parameters `name: &mut dyn Effect<...>`; effect member calls dispatch
  through the parameter (`console.print(...)` — auto-reborrow), and
  callee dependencies thread as arguments (`draw(random_int, console)`,
  with `&mut local` for handlers `use`d in the current scope). **When any
  handler in the program declares a dependency, all of this changes
  shape** — see [rs-effect-fusion].
  * Effect member parameters follow the default kept rule: `&T` for
    non-Copy, by value for scalars [rs-borrows]; member fns with their
    own generic parameters are a codegen error (`dyn` traits cannot
    have generic methods) [backend-never-wrong].
  * **Known gap** (2026-09-04): the default kept rule ignores the member's
    *declared* deductions, so a member that **moves** a parameter still
    emits `&mut T` and its body clones. Sound — the checker consumed the
    caller's value, so no alias can observe the copy
    ([effect-state-store] is what makes that true) — but a missed
    optimization. The fusion reshaped member *dispatch*, not member
    parameter modes, so this gap survived it.
* [effect-use] `use Handler(...)` emits
  `let mut <name> = Handler::new(args);` and registers `&mut <name>` in
  the effect environment for the rest of the scope [effect-scope]; ctor
  arguments are owned (a `use` argument is a move, [deduce-infer]).
  Handler generics are inferred by rustc from the `new` arguments (the
  checker already validated the instance, `use_effects`). Under the fusion
  the local is the *fusion* that owns the handler
  (`let mut __fx2 = __Fx_main_3 { __outer: &mut __fx, __h: H::new(args) };`)
  and every effect in scope threads through it from there on
  ([rs-effect-fusion]).
* Handler resolution prefers the checker's effect tables
  (`use_effects`/`effect_calls`/`call_effects`) rendered through
  `rust_ty`, with the same string-keyed environment fallback as the
  Kotlin backend (see PROGRESS.md "Emitter effect-environment fallback"
  under architectural facts).

### Effect dependencies via handler fusion [rs-effect-fusion]

**Implemented 2026-09-04.** A handler may declare a dependency
([effect-handler-deps]); Rust renders it by *fusing* the handlers of a
scope into one value. The user decision was "B9, rebuilding, no facets in
Rust" (2026-09-03/04); PROGRESS.md's E1a holds the decision log and the
rejected alternatives. Every shape below was verified by compiling and
running it with `rustc` before the emitter was taught to produce it.

**The problem.** A dependent handler must reach its dependency when its
member runs, without the *caller* of that member supplying one. Kotlin
gets this free (objects alias); Rust does not, because mutable state has
one usable path at a time and handler members are `&mut self`
([effect-decl]).

**The governing fact** is *borrow duration*, not reachability. A `&mut`
passed as an argument is a **reborrow** whose lifetime is the call: the
lender is suspended for exactly that long, then resumes. A whole chain of
frames may therefore reach one value while only the innermost uses it. A
borrow *stored* in a struct instead lasts as long as the holder, so it
overlaps the lender's own later use — `E0499`. Everything below follows
from putting borrows in the first category.

#### The emission

**Gated program-wide.** If no handler declares a dependency, nothing here
runs and effects thread as one `&mut dyn E` parameter each [rs-effects] —
existing output is untouched. The switch cannot be per-scope: a fn's
signature must not depend on which of its callers holds a fusion.

**One fused parameter per fn.** A fn needing one effect keeps
`__fx: &mut dyn E`; a fn needing two or more takes a *generic* fused
value:

```rust
pub fn banner<__Fx: Console + Logger>(__fx: &mut __Fx) { … }
```

Generic rather than `dyn` for one reason, and it is the reason worth
remembering: a fn must be able to forward its fused value to a callee
needing a **subset** of its effects (`shout(&mut *__fx)` for
`[Console]`). With `dyn`, that needs `&mut dyn Conj_A_B_C` →
`&mut dyn Conj_A_B`, which trait upcasting *cannot* do — upcasting only
reaches supertraits, and a conjunction of two is not a supertrait of a
conjunction of three. A Sized generic unsizes to any of its bounds, so
every subset call is trivial. The cost is monomorphization per fusion
type; it stays finite because fusion structs are not generic in their
provider (see below).

**One fusion struct per `use` site**, chained to whatever provided the
effects already in scope:

```rust
pub struct __Fx_main_3<'a, __H> {
    __outer: &'a mut dyn Console,   // or `dyn __Conj_…` for two or more
    __h: __H,                       // the handler this `use` registers
}
```

* **One `__outer` field**, not one per inherited effect: N reborrows of
  the same provider would alias. That single field is the only place a
  fused value needs a *nameable* type, and therefore the only reason
  conjunction traits exist:
  `pub trait __Conj_A_B: A + B {} impl<T: A + B + ?Sized> __Conj_A_B for T {}`
  — emitted per file that needs one, where the blanket impl makes
  duplication harmless (a fusion built in module `M` satisfies
  `N::__Conj_A_B` too).
* **`dyn` in `__outer`** keeps monomorphization finite: a recursive fn
  that registers a handler and recurses maps its fusion type to itself
  instead of nesting `__Fx<__Fx<…>>` forever.
* **Generic over the handler** (`__H`), so a *generic* handler needs no
  re-derivation of its type arguments: `CyclicRandom::new(vec![…])` infers
  them, and the fusion's impls bound `__H` by the effect it provides.
* The fusion **owns** the handler, so there is no separate handler local
  and no second borrow to manage.

**Chaining, not flat rebuilding.** The 2026-09-04 decision said inner
scopes rebuild *flat* over the outer scope's handler locals; implementation
showed that flatness cannot hold and is not what the decision was
protecting. Two facts forced the change: effects inherited from a fn's
*parameter* are not handler locals at all (there is only the one fused
value), and an inner fusion borrowing the same locals as an outer one
makes the **outer** fusion unusable after the inner block (`E0499`) —
exactly the case nesting is supposed to allow. Chaining fixes both, keeps
the property the decision actually wanted (dependency threading is
identical at every depth: always `&mut **__outer`), and makes lexical
nesting equal borrow nesting. The price is one dynamic forwarding hop per
level.

A dependency is therefore *always* reachable through `__outer` and never a
sibling field: it had to be registered before its dependent
([effect-handler-deps]), so it is always in the outer set. That is the
same acyclicity guarantee the strategy rested on, now doing a second job.

**Dependent handlers.** The dependency is neither a struct field nor a
`new` parameter — the compiler supplies it per call. The member bodies
cannot live in `impl Effect for H` (the trait signature has no room for
it), so they move into a generated trait:

```rust
pub trait __Impl_ConsoleLogger {
    fn log(&mut self, __fx: &mut dyn Console, message: &String);
}
impl __Impl_ConsoleLogger for ConsoleLogger { /* the written body */ }
```

`&mut self` is kept, so `self.state` still works, and the trait is only
ever a *bound* — never `dyn` — so its methods may be generic. A single
dependency travels as `&mut dyn D`; two or more need a Sized generic
(`__Fx: D1 + D2`), because a `dyn` cannot satisfy a Sized bound and a
`?Sized` one could not be unsized again further down. The Sized value is
built by a per-handler adapter over the one provider field:

```rust
pub struct __Deps_CountingAudit<'a, __P: ?Sized> { __p: &'a mut __P }
impl<'a, __P: Console + ?Sized> Console for __Deps_CountingAudit<'a, __P> { … }
```

The fusion's forwarding impl is where the trick lands — `&mut self` is
destructured into **disjoint field borrows** first, so the handler's state
and its dependency are two separate `&mut`:

```rust
impl<'a, __H: __Impl_ConsoleLogger> Logger for __Fx_main_3<'a, __H> {
    fn log(&mut self, message: &String) {
        let Self { __outer, __h } = self;
        __Impl_ConsoleLogger::log(__h, &mut **__outer, message)
    }
}
```

Independent handlers keep today's `impl Effect for H`, and the fusion
forwards to `&mut self.__h`.

**Member dispatch is UFCS** in fusion mode (`Console::print(recv, m)`,
`Random::<i32>::next_random(recv)`): one value implements every effect in
scope, so plain method syntax would be ambiguous between two effects with a
same-named member, and between two instances of a generic effect.

**Arguments that reach the fused value are hoisted** into a temporary:

```rust
{ let __a1 = &(format!("drew {} at {}", Counter::total(&mut __fx2))); println(&mut __fx2, __a1) }
```

`fx.a(&fx.b())` is two overlapping `&mut` (`E0499`). With per-effect
parameters the two receivers were disjoint variables, so this hazard is new
with the fusion — and it is why the emitter renders such calls as block
expressions.

**No facets in Rust** (user decision 2026-09-04): each backend leverages
its own language. Rust has no erasure, so one fusion implements
`Random<i32>` and `Random<String>` directly and multi-instance generic
effects need no mangling. Kotlin cannot ([kt-effect-fusion]).

#### Deliberate cuts inside the fusion ([backend-never-wrong])

* ~~A **function value that uses an effect, passed to a callee that needs
  one too**.~~ **Lifted 2026-09-04** by [fn-effects]: the effect is threaded
  *into* the value instead of captured, so the closure holds no borrow and
  the two borrows are of different things. The cut's own test program now
  compiles and runs. (What it was: a captured borrow stayed alive while the
  call borrowed the same fused value for its own effects — `E0499` — and
  hoisting, which rescues every other argument, could not separate them.)
* A **dependent handler that uses its own generic parameters** in a member
  signature: the generated `__Impl_H` trait is not generic (the fusion owns
  the handler behind an opaque `__H` and never derives its type arguments,
  so it could not supply one). Reported, naming the handler and the
  parameter. Kotlin accepts these (erasure), so this is a real divergence,
  and the way to lift it is to derive the handler's arguments at the `use`
  site by unifying its `of` clause against the checker's instance.
* A `use` whose **effect instance is still generic** — a handler whose type
  arguments the checker could not infer (`use Relay<Int>()`), or an effect
  list generic in the enclosing fn (`fn f<T>() [Random<T>, use]`). The
  fusion names its effects in impl headers, so an unresolved `T` would be
  an undeclared type. Lifting this means threading the enclosing fn's
  generics into the generated items.
* A generated **conjunction trait name claimed by two different effect
  sets**: sanitizing `<`/`,` to `_` is not injective, so an effect literally
  named `Random_i32` collides with `Random<i32>`. Vanishingly unlikely, but
  silently reusing the wrong trait would be wrong code.

Both are reported at the `use`/handler that causes them, never mis-emitted.

## Functions and calls

* [fn-dot] Dot-notation calls resolve to a declared fn/define/effect
  member and normalize to `f(base, args)`. There is no method-call
  fallback: an unresolved name here is a *codegen error* naming an internal
  inconsistency, since the checker already rejects undeclared dot-calls
  ([call-resolve]).
* [fn-variadic] Non-spread trailing arguments collect into `vec![...]`;
  a spread argument `...xs` forwards the vector (owned rendering).
* [fn-lambda] Lambdas emit as closures (`|a, b| expr`); fn-typed
  parameters emit as `impl Fn(A, ..) -> R`. Lambda parameters are owned.
  Early `return` inside expression-position lambdas remains a codegen
  error (same cut as Kotlin).
* [backend-define-inline] Define templates expand inline at call sites
  with the *owned* rendering of each argument, except that a template
  parameter declared `Mut` receives a mutable place (the raw argument) —
  method-style templates (`${list}.push(${elem})`) then borrow the place
  natively. `imports:` lines hoist per generated file.
* [backend-define-generics] `${T}` in a `define fn` expands to the call's
  resolved type argument, as on Kotlin. The Rust templates deliberately do
  *not* use it where rustc infers as well as the checker knows: `vec![]`
  stays `vec![]`, since [call-type-args] guarantees the element type is
  either written or annotated, and both reach rustc through the rendered
  `let` annotation or parameter type. The capability is there for a define
  whose target construct needs the type spelled out
  (`Vec::<${T}>::new()`).
* [intrinsic-fn] Every std lowering lives in this crate's `intrinsics.rs`,
  keyed by the checker-resolved declaration (name + first parameter's base
  type name, so `size(Str)` / `size(List<T>)` / `size(T[])` are three
  entries). `copy` [rs-copy] and `discard` are the exceptions: they
  dispatch on the argument's own shape, so `emit_intrinsic_call` handles
  them before consulting the table. An intrinsic with no entry is a codegen
  error naming it.
  * The argument boundary keeps the place/owned distinction the templates
    relied on [rs-borrows]: a place splices raw so `list.push(..)` borrows
    natively, while a variadic tail splices owned because it lands inside
    `vec![..]`. Backwards, this either double-clones or moves out of a
    borrow.
  * Paths are absolute, so no lowering adds a `use` item.
* [rs-platform-entry] [platform-effect] A `platform effect` emits the same
  `trait` an ordinary effect does and threads as `&mut dyn` in the same way,
  but **no** handler struct — the host writes the impl. A `main` that
  declares one is emitted as `salvo_main` taking the instances
  (`SALVO_ENTRY`); Rust requires `fn main` in the crate root, and the host's
  is the one that belongs there. Only *platform* effects become `main`'s
  parameters; everything else it needs is registered inside it with `use`.
  * An `intrinsic handler` is the mirror image: struct + `new()` + trait
    impl, with member signatures from the *effect* declaration and bodies
    from `intrinsics::handler_member`.
* [rs-copy] `copy(x)` lowers to `.clone()` on the argument's place:
  a bare identifier clones its binding place (whatever its binding
  mode — every generated type derives or is `Clone`, and generic
  parameters carry a `Clone` bound); a narrowing-unwrapped identifier
  uses the unwrap rendering (already an owned clone); field/index
  arguments use the owned rendering (already a clone); constructed
  values (call results, literals) pass through — they are already
  fresh, so `copy` is free on them.
* [struct-defaults] Rust has no default arguments: struct literals
  inline the declared default expressions for omitted fields at every
  literal site.
* [fn-contract] Fn-typed parameters emit `&mut impl FnMut(…)` — the
  value is borrowed (closure double-use works; `FnMut` accepts
  handler-mutating closures), with argument types per the contract:
  kept non-Copy `&T`, kept `Mut` `&mut T`, moved or Copy owned. Calls
  through fn values render arguments per the recorded contract
  (`Checked::fn_value_calls`); lambda parameter bindings and
  annotations follow `Checked::lambda_contracts`; a named fn passed by
  value wraps in a mechanical adapter closure
  (`&mut |__a0, …| name(&__a0, …)`) bridging the contract's calling
  convention to the declaration's actual modes.
* [once-fn] `Once` fn parameters emit `impl FnOnce(…)`; consuming
  closures are `FnOnce` by rustc's own capture inference, so lambda
  emission is unchanged. Calling the parameter is a plain call (the
  by-value `call_once` is implicit).
* [linear-discard] `discard(x)` lowers to `drop(x)` on the moved value
  [intrinsic-fn]; linearity itself is purely static [linear-static] — no
  `#[must_use]`, no `Drop` impls are generated.
* [struct-spread] `P {...p, f: v}` emits
  `P { f: v, ..(p-owned) }` (functional update; the base is rendered
  owned, cloning when needed). The deep clone diverges from Kotlin's
  shallow `.copy()` on `Mut` fields, which is unobservable because the
  checker consumes the spread base [deduce-consume] — replacing the
  clone with a real move is a deferred performance refinement.

## Running the output [rs-run]

* [cli-run] [rs-run] `salvo run --backend rust` builds the crate root with
  `rustc --edition 2021 <target>/<root>.rs -o <target>/.salvo_bin/<name>`
  and runs the binary. One invocation is enough: the root reaches every
  other emitted module through its `mod` declarations [rs-crate].
  * The binary directory is dot-prefixed so a target nested in the source
    tree stays invisible to source discovery [mod-ignore].
  * **The chosen entry must reach the emitter**, because the crate root
    *is* the `main`-declaring module: with several candidates the emitter
    would otherwise pick the first it found, and building any other one
    leaves the `mod` declarations behind — rustc then reports unresolved
    imports for every module. This is why `Backend::emit` takes the
    entry.

## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* multi-spread struct literals;
* early `return` inside expression-position lambdas;
* struct literal without an inferable type;
* referencing the `Any` type in emitted positions;
* effect member fns with their own generic parameters;
* a dependent handler using its own generic parameters in a member
  signature, and a `use` whose effect instance is still generic
  ([rs-effect-fusion]);
* struct destructuring in `for` patterns (same as Kotlin).

Known acceptable divergences (documented, not errors): eager iterator
functions [rs-iter-vec]; extra `.clone()`s where Kotlin shares
references; `Debug`/`Display` formatting of `Option` values differs from
Kotlin's `null` printing (the checker's narrowing rules make user
programs format only unwrapped values).
