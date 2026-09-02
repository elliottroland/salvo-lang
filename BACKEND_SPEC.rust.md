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
* [type-str] Strings are `String` (owned). Plain string literals emit
  `"...".to_string()`; interpolation emits `format!("{}...", args)`.
* [type-alias] Aliases expand structurally in the emitter (same
  `subst_ast_type` approach as Kotlin).
* [qual-erasure] Qualifiers erase from emitted types; what survives is
  arm choice, casts, predicate calls, mangled names — and the borrow
  modes that `Mut` implies [rs-borrows].
* [type-with-mut] Rust maps `Mut T` to the *same* type as `T` (no
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
* **Lifetimes.** No emitted signature *returns* a reference and no
  emitted struct *stores* one (results are owned; struct fields are
  owned), so every function is lifetime-elision-friendly and no named
  lifetimes are ever generated.
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
  * Narrowed ident uses unwrap in place: `x.u2().clone()`
    (`x.as_ref().unwrap().u2().clone()` for a nullable repr). Unlike
    Kotlin, Rust also unwraps a `T?` repr narrowed to its value arm:
    `x.unwrap()` for Copy scalars, `x.as_ref().unwrap().clone()`
    otherwise — there is no smart cast to lean on.
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
* [if-bool] `if` is an expression in both languages; a missing `else`
  on a value-position `if` emits `else { None }` ([if-else-none], the
  branch values carry `WrapOption` coercions). Statement-position
  branches emit their tails as statements.
* [loop-while-is] `while x is T (name)?` re-tests in the loop condition
  and re-binds per iteration at the top of the body (same shape as
  Kotlin).
* [rs-postincrement] Rust has no `++`: statement-position `i++` emits
  `i += 1;`; value-position emits `({ let __t = i; i += 1; __t })`.

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
  `&mut self` (handlers are stateful).
* [effect-handler] Handlers emit as `pub struct H<..> { ctor-params,
  state }` + `impl H { pub fn new(ctor-params) -> Self }` (state fields
  initialized from their declared defaults) + `impl Effect for H`.
  Handler member bodies access ctor params and state through `self.`.
  External handlers inline their `define handler` templates as method
  bodies, same shape.
* [kt-effect-params]-equivalent: effect dependencies become leading
  parameters `name: &mut dyn Effect<...>`; effect member calls dispatch
  through the parameter (`console.print(...)` — auto-reborrow), and
  callee dependencies thread as arguments (`draw(random_int, console)`,
  with `&mut local` for handlers `use`d in the current scope).
  * Effect member parameters follow the default kept rule: `&T` for
    non-Copy, by value for scalars [rs-borrows]; member fns with their
    own generic parameters are a codegen error (`dyn` traits cannot
    have generic methods) [backend-never-wrong].
* [effect-use] `use Handler(...)` emits
  `let mut <name> = Handler::new(args);` and registers `&mut <name>` in
  the effect environment for the rest of the scope [effect-scope]; ctor
  arguments are owned (a `use` argument is a move, [deduce-infer]).
  Handler generics are inferred by rustc from the `new` arguments (the
  checker already validated the instance, `use_effects`).
* Handler resolution prefers the checker's effect tables
  (`use_effects`/`effect_calls`/`call_effects`) rendered through
  `rust_ty`, with the same string-keyed environment fallback as the
  Kotlin backend (see PROGRESS.md "Emitter effect-environment fallback"
  under architectural facts).

## Functions and calls

* [fn-dot] Dot-notation calls that resolve to a known fn/define/effect
  member normalize to `f(base, args)`. Unknown methods emit as Rust
  method calls (`base.f(args)`) for companion-code interop
  ([type-unknown-lenient]).
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
* [internal-fn] Internal fns bypass define templates: the emitter lowers
  the call directly. Rust implements `copy` [copy-fn] as [rs-copy]; any
  other internal fn is a codegen error.
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
* [linear-discard] `discard(x)` lowers to `drop(x)` on the moved value
  [internal-fn]; linearity itself is purely static [linear-static] — no
  `#[must_use]`, no `Drop` impls are generated.
* [struct-spread] `P {...p, f: v}` emits
  `P { f: v, ..(p-owned) }` (functional update; the base is rendered
  owned, cloning when needed). The deep clone diverges from Kotlin's
  shallow `.copy()` on `Mut` fields, which is unobservable because the
  checker consumes the spread base [deduce-consume] — replacing the
  clone with a real move is a deferred performance refinement.

## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* multi-spread struct literals;
* early `return` inside expression-position lambdas;
* struct literal without an inferable type;
* referencing the `Any` type in emitted positions;
* effect member fns with their own generic parameters;
* struct destructuring in `for` patterns (same as Kotlin).

Known acceptable divergences (documented, not errors): eager iterator
functions [rs-iter-vec]; extra `.clone()`s where Kotlin shares
references; `Debug`/`Display` formatting of `Option` values differs from
Kotlin's `null` printing (the checker's narrowing rules make user
programs format only unwrapped values).
