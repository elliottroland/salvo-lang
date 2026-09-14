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
    `mod unions` [rs-union-enums], and the string helpers in `strings.rs`
    [rs-mut-str], each emitted only when the program needs it. (The lazy
    `iter.rs` runtime went with the `yield fn` deletion, 2026-09-10:
    iteration emits inline pass drives [rs-iter-pass], no runtime file.)
* [rs-imports] Files get generated `use` items: `use crate::<mod>::*;`
  per foreign *emitted* module whose names the file uses, and
  `use crate::unions::*;` / `use crate::strings::*;`
  when the file touches union wrappers or the string helpers. An aliased
  Salvo import of a Rust-visible item emits
  `use crate::<mod>::<name> as <alias>;` and call sites keep the alias.
  Intrinsic lowerings name everything by absolute path [intrinsic-fn], so
  they add no `use` of their own.
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
* [fn-overload-at] [fn-rename] Both caller-side overrides of overload
  resolution are **erased**: `call_fn` records the declaration and mangling
  keeps Rust from re-resolving it [rs-fn-mangling]. A renamed call emits the
  declaration's own name (`renamed_calls` tells it apart from an import
  alias, which is kept [rs-imports]).
* [rs-shadowed-call] A call that reaches past a **local of the same name**
  (only possible as `f@module(...)`, since a plain call would have gone
  through the local) is spelled as a path — `crate::<mounted module>::f(...)`,
  or `crate::f(...)` for the crate root. Rust puts functions and locals in
  one value namespace, so the bare name is the local (E0618: "call expression
  requires function"); Kotlin needs nothing, which is why this rule is
  backend-prefixed.
* [rs-seq] std's sequence functions [seq-pass]: the `List` fast paths
  lower to the generated helpers in `strings.rs`'s sibling `seq.rs` —
  `salvo_map`/`salvo_filter`/`salvo_reduce`, taking `&[T]` so a call splices
  its receiver as `&place[..]` and works for an owned `Vec`, a `&Vec` and a
  `&mut Vec` alike.
  * **Functions rather than inline expressions**, and the reason is closure
    inference: a Rust closure bound to a `let` cannot infer its parameter
    types, and neither can one nested in another closure's argument, so
    every inline shape needed an annotation the emitter does not have. A
    generic parameter *is* an expected type — and it also pins the
    callback's convention (`FnMut(&T)`), which is what a fn-typed parameter
    of declared type `(T) -> U` renders as [rs-fn-param-convention].
  * `salvo_reduce` is a loop, not `Iterator::fold`: the callback's
    accumulator is *borrowed* by that convention and `fold` passes it by
    value.
  * A **lambda** argument to an intrinsic follows the declared parameter
    type's conventions like any other fn-typed position, and a **named fn**
    wraps in the same adapter closure [fn-contract] — a fn item's own
    convention is by value, which is E0631 against `FnMut(&T)`.
* [implicit-intrinsic] An `intrinsic fn` filling an implicit parameter is
  passed as an adapter closure whose body is the intrinsic's *lowering*:
  emitting `iter(__i0)` would name the generated `iter` **module** (E0423).
  A resolved *declared* fn's adapter forwards each argument in that fn's own
  parameter mode [rs-borrows] — a kept struct parameter is `&T`, and passing
  it by value is E0308.
* [rs-mut-str] `Str` and `Mut Str` are **both** `String`: mutability lives
  in the binding and the reference [type-canbe-mut], so a `Mut` drop
  renders nothing at all ([str-drop-mut] — `coercion_of` unwraps a
  `DropMut` record to whatever it carries, so even the "is this argument a
  fresh temporary?" tests see that nothing happens at a drop).
  * String indexes are **characters**, not bytes: `size` counts `chars()`,
    and `index_of`/`substr`/`set` convert, since `find` answers in bytes.
    (Kotlin counts UTF-16 code units — the divergence `size` already had.)
  * A read-only lowering binds its receiver once — `{ let __s = &A[..]; … }`
    — which both avoids evaluating a call argument twice and gives a `&str`
    whatever shape the place had (`String`, `&String`, `&mut String`).
  * `mut_str(parts)` *borrows* its parts (`[&a[..], &b[..]].concat()`):
    they are read, not stored, and a variadic position is untracked by the
    flow analysis, so an owned splice would move a variable the checker
    still considers live [fn-variadic].
  * `set` is the one operation with no single `String` method, and an
    inline `let s: &mut String = &mut place;` does not work for a `&mut
    String` *parameter* (E0596: the binding is not `mut`). So it is a
    method on a generated trait — `strings.rs`, mounted and imported like
    `iter.rs`, gated on use — which auto-refs every place shape and
    mentions the receiver once.
* [kt-none-unit]-equivalent: `None` as a return type is `()` (omitted);
  `None` as a union arm is `Option` [rs-option].
* [rs-option] `T?` maps to `Option<T>`: `None`→`None`, `is None`→
  `.is_none()`, `x!`→`.unwrap()`. Optionals are *physical* in Rust, so
  the checker's `WrapOption` coercion emits `Some(code)` [type-nullable]
  (Kotlin ignores the same coercion).
* [type-array] `T[]` maps to `Vec<T>`; `array_of` emits `vec![...]` and
  `array_by` an iterator-map-collect [col-by]; indexing casts the `i32`
  index (`v[(i) as usize]`).
* [rs-iter-pass] Iteration is **passes all the way down** [iter-protocol]:
  there is no iterator type and no runtime support module for one. A pass is a
  plain struct, `next` is a plain function, and a `for` over one is
  `while let Union2::U1(x) = next(&mut p) { … }` — an inlined call per element,
  no allocation, no trait object.
  * [iter-fn] An `iter fn` is desugared before emission into that same shape: a
    struct holding what the body reads of the subject plus its `state` fields, an
    `iter` that mints one, and the body as the `next`. The backend has no rule
    of its own for the form.
  * A `for` over a **container** calls its `iter` once before the loop and drives
    the result [iter-pass]; the intrinsic containers (`Vec`, arrays, `String`)
    keep their native loop instead [iter-for-native].
  * An **effectful `next`** takes its handlers as leading arguments, threaded
    into every turn of the loop from the scope the `for` is written in
    [fn-effects].
  * [linear-group] A pass with a `close` is released by the loop on every exit —
    exhaustion, `break` and `return` — through the exit-splice path
    [rs-exit-splice].
  * [rs-fn-field] A fn-typed **field** is `Rc<dyn Fn…>`, which is what lets a
    composed pass store its source's `next`; a fn that stores a
    callback therefore takes it owned and `'static` rather than borrowed. std
    stopped writing such a pass when the lazy pair was removed (2026-09-10);
    a program may still write one.

* [rs-implicit-turbofish] A **generic call that fills implicit parameters**
  spells out its type arguments (`map_to::<Vec<i32>, Vec<i32>, i32, i32>(…)`),
  from `Checked::call_type_args`. Each implicit arrives as an adapter *closure*
  whose parameter types Rust infers from the callee's bound, so with the
  callee's generics still open there is nothing to infer them from and
  inference stalls (E0282) on closures waiting for the answer the call itself
  would have given.
  * An implicit's fn type renders its parameters through the **contract**
    [fn-contract], not through the parameter types alone: a kept `Mut`
    position is `&mut T`, because a callback cannot append to a destination
    handed over by value. Narrowed to kept-`Mut` positions deliberately —
    everything else keeps the by-value convention implicits have always had.
  * An implicit a callee **keeps** follows the same convention a written stored
    callback does [rs-fn-field]: owned `impl Fn(…) + 'static` in the signature,
    `Rc`-held once stored, and passed by a `move` adapter at the call site. For
    a *minted origin* the adapter wraps the machine's own advance
    (`move |__p: &mut __Pass_X| match __p.__advance() { … }`), and its parameter
    is annotated because rustc cannot infer it through the `&mut dyn FnMut`
    coercion an unkept position renders.
* [rs-none-unit] `None` is Rust's `()`, and a fn returning it has no return
  type — so `return None` emits a **bare** `return`. The test is the *fn's*
  rendered return type, not the value's: `return None` in an
  `Option`-returning fn is `return None;` and correct. The literal is dropped
  rather than evaluated; any other `None`-typed value runs for its effects
  first.
* [type-tuple] Tuples map to native Rust tuples (any size).
* [rs-fn-field] A **function in a struct field** is an `Rc<dyn Fn…>`
  (roadmap R5, 2026-09-08 — it was a codegen error until then, while Kotlin
  accepted the same source: a live backend divergence, now closed). The
  representation is the one a *generated* pass has always used for a stored
  callback [rs-iter-pass], reused rather than reinvented:
  * the field renders as `std::rc::Rc<dyn Fn(A) -> R>` — `Fn`, not `FnMut`,
    because it is reached through a shared `Rc` (the same rendering an
    iterator fn's callback parameter gets, with `impl `/` + 'static`
    rewritten to `Rc<dyn …>`);
  * a **store** wraps in `std::rc::Rc::new(…)`, in a struct literal and in
    an inlined default alike. Wrapping a value that is *already* an `Rc`
    re-coerces (one more indirection) rather than failing, so the rendering
    does not depend on where the value came from;
  * the struct gets `#[derive(Clone)]` and a **hand-written `Debug`** that
    prints the callback as `<fn>`, since `dyn Fn` has no `Debug` and `{:?}`
    is how `${…}` renders a struct [rs-display];
  * a read is an ordinary place read (`Rc` clones), and calling the value
    still needs a local first (`let g = h.f` then `g(e)`) — `h.f(e)` is
    dot-notation for `f(h, e)` [fn-dot], which is a language rule, not a
    backend one;
  * what this buys: a **hand-written composed pass** (a struct storing its
    source *and* its callback) builds on both targets, so composing passes
    is available to anyone rather than only to generated std code.
* [backend-never-wrong] A **`params` group in a struct field** is still a
  codegen error: a bundle of functions has no single type to store. The
  checker refuses it first [group-not-a-value]; the backend keeps its own
  guard, and the diagnostic names what does work — an implicit parameter, or
  a `params` group in a signature [implicit-group].
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
  * **Refinements [qual-refn] reach output only through that list**, and
    notably *not* through the borrow modes: a refinement may only name
    *state* qualifiers, so it can never grant or revoke `Mut` and can never
    change a parameter's mode. It adds no call and no check either — a
    refined claim is trusted, so no `qualifies` call is emitted where one
    applies. Nothing to lower.
* [type-canbe-mut] Rust maps `Mut T` to the *same* type as `T` — there is
  no per-type `Mut` mapping at all. Unlike Kotlin's `intrinsics.rs`, the
  Rust one deliberately has no `mut_type_name`: mutability is expressed in
  bindings and references (`let mut`, `&mut`) [rs-borrows], not in the
  type. Struct `Mut` works the same way (all struct fields are plain
  fields; assignability is enforced by the checker).

## Ownership and borrowing [rs-borrows]

The central design (per LANGUAGE.md "Deductions"): the checker's deduction
tables (`Checked::deductions`, [deduce-syntax] [deduce-infer]) are the
ownership contract. Salvo source has no references; the Rust backend
derives them mechanically:

* **Parameter modes.** For each parameter of a fn with a deduction entry:
  * *consumed* (`=> !p`, or inferred so; `kept == false`) → the parameter is
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
    * **Field-disjoint borrows need nothing extra**
      [fate-field-disjoint]: since L5 the checker lets `let n = p.name`
      stay live across a mutation of `p.tags`, and that emits as a real
      borrow held across `&mut p.tags` — which rustc accepts, because
      they are disjoint fields of one local. The two analyses draw the
      same line (same field, prefix, whole variable and computed index
      all still poison), so the newly legal programs compile clone-free
      with no emitter change. The **move** half matches too since
      2026-09-10 [fate-partial-move]: a moved projection emits as a real
      partial move of the field and a later reassignment as rustc's
      reinitialization, both borrowck-legal, with the whole-value use
      refused on the Salvo side before it can reach rustc.
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
  calls); std's `first` intrinsic lowers to `list.first()` — clone-free.
* **Generic bounds.** Every generic parameter gets a `Clone` bound
  (`<T: Clone>`) — the owned-rendering rule may clone values of generic
  type. Structs additionally `#[derive(Clone, Debug, PartialEq)]` —
  equality works on every struct [col-equality] — plus `Eq` and `Hash` for
  `canbe hashed`, and `Eq, PartialOrd, Ord` for `canbe ordered`
  [col-hashed-ordered]. A struct with a fn-typed field derives only `Clone`:
  `Rc<dyn Fn>` has neither `Debug` nor equality [rs-fn-field].
  Generated union enums derive `PartialEq` too, conditionally on their
  payloads, so a struct holding one can derive its own.
* Deliberate simplicity, accepted costs: kept-parameter arguments are
  never moved even when it would be their last use (a clone happens
  instead); rustc's borrow checker remains the final authority — a
  program that emits but does not borrow-check is a compiler bug, not a
  user error.

### Projections [rs-proj]

The `proj` rules of LANGUAGE_SPEC.md ([proj-type] [readonly-return]
[proj-anywhere] [proj-readonly] [proj-field] [proj-infer] [yield-proj])
are the one place Salvo's source states a borrow, and this backend has
**one rendering rule**: `proj X` *is* `&X` (2026-09-12, following
[proj-type]) — at whatever depth the projection sits: a union arm
(`Union2<&String, Finished>`), a type argument (`Vec<&String>`), a struct
field, a parameter, a return. Under a named lifetime context it is
`&'s X` / `&'a X`. Everything below is the machinery that names the
lifetimes and adapts call sites; nothing here clones. Two exceptions to
the blanket rule:

* A Copy scalar's `proj` never reaches the emitter — the checker erases
  it ([proj-type], [copy-scalar-free]).
* `proj T` over a **bare generic parameter at its definition site**, with
  no lifetime context, renders owned `T`: a generic body treats `T`
  uniformly, and whether a use borrows is the instantiation's fact — the
  caller substitutes `T = proj Str` (rendering `&String`) and the
  turbofish retag spells it [rs-proj-arm]. Rendering `&T` at the
  definition would borrow for every instantiation, owned ones included.
  (Inside a borrowing struct or its `next`, where `'s` is in scope, a
  generic projection *does* render `&'s T` — the struct's own borrow.)

* [readonly-return] A wholesale projection returns `&T`, `Option<&T>` or
  `Union2<&T, Finished>`. One reference parameter: lifetime elision. More:
  `'a` is generated onto **every** source parameter (`proj[from: a, b]`)
  and the return. A returned projection of a `&mut` pass parameter that is
  itself a borrowing struct names the *struct's* source lifetime instead
  (`next(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished>`
  [rs-proj-struct]), so the reborrow of `p` is free for the next turn.
* [rs-opt-borrow] A local bound from an optional projection (`let h =
  first(xs)`) holds `Option<&T>` (`BindKind::OptRef`): a later narrowing
  unwraps the reference (`*h.unwrap()` for Copy, `h.unwrap().clone()`
  owned) rather than cloning the reference itself; in a `&T` position it
  passes `h.unwrap()`; `let v = get(xs, i)!` binds as a plain `Ref`.
  Found live 2026-09-11: the generic path emitted `.as_ref().unwrap()
  .clone()`, a clone of the *reference* (`&&T → &T`).
  * [proj-type] [lambda-view] An unwrap whose own checker type is a
    projection stays the reference — no clone, no deref: a lambda tail
    `get(all, i)!` typed `proj Str` yields `&String` into the closure's
    return (`Vec<&String>` at the call), and an interpolated
    `first(names)!` displays through the reference. Any position that
    truly needs ownership was checker-refused without `copy` before
    emission (2026-09-12; before, every non-Copy unwrap in a value
    position cloned — a copy the program never opted into).
  * A Copy scalar read out of a reference binding (a lambda parameter
    under the `FnMut(&T)` convention [rs-fn-param-convention]) renders
    as a *value* in intrinsic argument positions (`*i`): lowerings use
    scalars in casts (`(i) as usize`), which a `&i32` place fails
    (E0606; found live 2026-09-12).
* [rs-proj-struct] A struct with a `proj` field — or an owned field whose
  type has one, transitively — is a **borrowing struct**: `struct
  ListYield<'s, T> { items: &'s Vec<T>, at: i32 }`, with `<'s>` on owned
  view-typed fields; every mention elides (`ListYield<'_, T>`); a struct
  literal borrows into its `proj` fields (`&list`); it is returned *by
  value* (the struct carries the lifetime, no `&` wraps it).
* [rs-proj-lends] The lifetime a view carries reaches the parameters it
  borrows [proj-infer]: with one reference parameter elision ties them;
  with more, `'a` is named on every lent parameter (`Checked::fn_lends`)
  and the return. A **lent implicit position** (`?iter: (c: C) -> Mut It`
  with `=>[iter] proj[from: c]`) renders `&'c C` under a lifetime `'c`
  named on the enclosing fn's kept parameter `c` — the result's type
  (`It`) is fixed at the call site, so the borrow it holds cannot be a
  fresh per-call one; the enclosing fn must keep `c` (a consumed one has
  nothing a view could outlive — reported). Re-pointing entries
  (`v.items: proj[from: other]`) tie `'r` on the target struct and the
  source parameters, and the assignment renders as a borrow.
* [rs-proj-arm] A union with a `proj` arm is an ordinary instantiation of
  the shared enum with a reference arm (`Union2<&'s T, Finished>`). At a
  call filling `?Yield<It, T>` from a borrowing `next`, the element generic
  is **retagged** to `&T` in the turbofish — unless the checker's
  substituted type already carries the projection (`T = proj Str` renders
  `&String` on its own [proj-type]), in which case the retag defers; user callbacks at a retagged
  position arrive one reference deeper and peel it (`let n = *n;` at the
  top of a lambda, `let __a0 = *__a0;` in a by-name adapter; an annotated
  lambda parameter renders `&&T`); resolved implicit adapters clone a
  retagged position out where the callee owns it (`push`), and `copy` at
  a retagged position is the identity. A concrete Copy element
  (`?Yield<It, Int>`) is copied out by a match adapter instead. A local
  bound from a `proj`-arm call remembers its borrowed arms
  (`borrowed_arm_locals`): a payload read of one derefs twice for a Copy
  scalar and binds as `Ref` otherwise; such a value flowing into a
  position written as the owned union is adapted arm by arm for Copy
  payloads and is a codegen error otherwise (a hidden clone this backend
  refuses).
* Views of temporaries are refused by the checker [proj-anywhere]; the
  only thing this backend adds is that rustc would have said the same
  (E0716).

## Unions [rs-union-enums]

* [kt-union-wrappers]-equivalent: wrapper unions emit as generated
  enums in `unions.rs`:
  `pub enum UnionN<T1..TN> { U1(T1), .., UN(TN) }` with
  `#[derive(Clone, Debug)]`, per-arm accessor methods
  (`pub fn u1(&self) -> &T1` and `pub fn u1_mut(&mut self) -> &mut T1`,
  panicking on the wrong arm — unreachable when the checker's tables are
  right; the `_mut` half is what a mutable use of a narrowed value needs
  [rs-narrow-mut]), and a `Display` impl (bounded on
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
* [rs-narrow-mut] A **mutable** use of a narrowed place unwraps through the
  mutable accessors instead, so the result is a `&mut` *into the storage*:
  `p.as_mut().unwrap()` for a nullable repr, `q.u1_mut()` for a wrapper-union
  arm (the generated enums carry `pub fn ui_mut(&mut self) -> &mut Ti`
  alongside `ui`), and the two compose (`x.as_mut().unwrap().u1_mut()`).
  Two sites need it: an argument in a `&mut` position, and the **base** of an
  assignment target (`r.at = 2` on a narrowed `Mut ListYield<Int>?` becomes
  `r.as_mut().unwrap().at = 2`). The outermost node of an assignment target
  keeps the read rule — assigning to a narrowed *variable* writes its
  storage, not through the narrowing.
  * **Why it is a rule and not an optimization**: reusing the read form was
    silently wrong. `&mut (p.as_ref().unwrap().clone())` compiles, and the
    mutation lands on the clone — a pass driven through a narrowed handle
    re-emitted its first element for ever, while Kotlin (whose smart cast
    *is* the storage) advanced. Fixed 2026-09-10; it is the one
    [backend-never-wrong] violation this compiler has shipped.
  * The read and mutable unwraps share one classification of the narrowing
    (`Narrowing`: arm index, whether an `Option` sits in front, whether the
    payload is `Copy`), so they cannot disagree about arm identity
    [union-arm-identity].
  * The gate is the unwrap *producing* something, not the presence of a
    recorded representation: a `Mut` drop records one too, and a bare name
    still needs its `&mut`.
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
* [rs-inc-dec] Rust has neither `++` nor `--` [inc-dec], so:
  * statement position is a compound assignment — `i += 1;` / `i -= 1;`,
    the same for both fixities since the value is discarded;
  * value position is a block: postfix `({ let __t = i; i += 1; __t })`
    (the old value), prefix `({ i += 1; i })` (the new one).
* [rs-interp-to-str] A non-native interpolated value [interp-to-str] is
  wrapped in the `to_str` the checker resolved: an `intrinsic` one goes
  through its lowering template with a pre-rendered argument, a declared one
  is an ordinary call on a borrow. A derived struct [interp-struct] renders
  as a nested `format!` over its fields — *not* `{:?}`, which would quote
  strings and so disagree with Kotlin.
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
* [rs-exit-splice] Code the compiler owes at a block's exits is **spliced**:
  emitted at every exit of that block, leaving no runtime construct behind.
  Rust has no `finally`, and a `Drop` guard cannot stand in: a release
  consumes its handle, so the guard would have to own it from registration
  onward, making it unusable for the rest of the block (a `&mut` capture
  trades that for `E0499` at the next use).
  * The only source today is the release a `for` owes a pass it owns
    ([linear-group]; the `defer` statement was the other until it was removed
    from the language, 2026-09-10).
  * The code is rendered **once**, where it is registered (in that scope, with
    that point's effect environment and bindings), and re-indented at each
    splice site. Sites: the end of the block (`emit_block_stmts`, skipped when
    the block's last statement already exits — the splice would be dead code
    rustc still borrow-checks), each `return` (every splice of the fn,
    `splice_floor` stopping at a closure boundary), and each
    `break`/`continue` (those registered inside the loop, tracked by
    `loop_splice_floors`).
  * A value given away at the exit is computed first, so `return v` with a
    splice behind it becomes
    `let __exit_valueN = v; <splice>; return __exit_valueN;` — and a
    value-position block hoists its tail the same way (`emit_value_block`),
    since the spliced statements would otherwise become the block's value.
  * **Known divergence** from Kotlin's `finally` lowering (accepted, user
    decision 2026-09-04): a panic out of a std intrinsic unwinds *past* the
    splice, so the code does not run on a crash path, where the JVM's
    `finally` would run it. See [kt-exit-finally].
* [throw] [rs-throw-controlflow] A fn declaring `[Throw<M>]` returns
  `ControlFlow<M, T>` — the message type *is* `ControlFlow`'s `Break`
  payload, so the propagation falls out of the design rather than being
  imposed on it. `Throw` is filtered out of the effect *parameters* (it is a
  return shape, not a capability), and the effect declaration itself emits
  no trait: it has no handlers to implement.
  * `throw(m)` is `return ControlFlow::Break(m)` — no handler, no dispatch,
    no allocation. `return v` becomes `ControlFlow::Continue(v)`, and a
    `None`-returning fn ends with `return ControlFlow::Continue(())`.
  * A call that may throw unwraps with `?` — but only when the throw would
    leave *this* fn unchanged. Inside a `try`, when the message must be
    wrapped into a union arm, or when an exit splice has to run first, the
    propagation is written out as `match call { Continue(__v) => __v,
    Break(__m) => <transfer> }`. That is an *expression*, so it works in
    argument position with no hoisting — and it is the only way a pending
    release can run on the throw path, since `?` returns without it.
  * Verified by hand before implementation (`?` on `ControlFlow` is stable;
    a may-throw call in a loop stays a loop, no trampoline): the three
    compositions the roadmap asked for are in the E3 notes.
* [try] [rs-try-label] `try { ... }` is a **labelled block**
  (`'try_N: { ... }`), not a closure. The sketch's closure
  (`(|| -> ControlFlow<..> { .. })()`) would have to capture the fn's
  effect parameters and any local the body mutates — the same exclusivity
  trap the fusion hits; a labelled block captures nothing.
  * The price is that `?` cannot be used inside a `try` body (it would
    return from the *fn*), so every may-throw call there takes the `match`
    form above and `break`s the label with the outcome's thrown arm. The
    body's tail is wrapped into the `Ok` arm (arm 0); a body that always
    leaves still needs a value for the block, which the checker made
    `Ok None` [try].
  * Nesting needs no token: the label decides where a `break` lands, so an
    inner delimiter cannot swallow an outer throw [try-innermost].

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
  mangling gap. Kotlin applies the identical rule for a different reason
  — it *has* overloading, and would resolve by its own lattice
  [kt-fn-mangling].
* [implicit-param] [implicit-resolve] An implicit parameter emits as an
  ordinary trailing parameter, rendered like any fn-typed one:
  `&mut impl FnMut(..) -> R` [fn-contract]. The call site passes what
  resolution found — a resolved default as a mechanical adapter closure
  (`&mut |__i0, __i1| add(__i0, __i1)`, since a fn item is not a closure),
  a forwarded one as a reborrow (`&mut *add`), an override as the written
  value (adapted the same way when it names a fn).
  * A `params` group emits nothing [implicit-group]: it never was a value, so
    there is no struct and nothing boxed.
  * A **kept-`Mut`** parameter of an implicit position is rendered `&mut T`
    (`fn_ty_param_renderings`), so the adapter closure passes it **straight
    through** rather than borrowing it again: `&mut |__i0| next(__i0)`, not
    `next(&mut __i0)` (`E0596` — you cannot take `&mut` of a `&mut` binding).
    A callee wanting `&T` takes the same value (`&mut T` coerces); one wanting
    it owned clones. First reachable with `params Yield`'s `next`, which is
    the only member so far that mutates its subject.
  * An implicit parameter is `&mut dyn FnMut(..)` — **`dyn`, uniformly**, for
    two reasons that pull the same way. An effect member's implicits land in a
    trait used as `&mut dyn E` [rs-effects], where `impl Trait` in argument
    position would cost object safety. And forwarding has to compose in every
    direction: a member forwarding to a plain fn would otherwise hand a `dyn`
    value to an `impl` (`Sized`) parameter, which rustc refuses. One
    convention is both simpler and the only sound choice; the cost is an
    indirect call, which every effect member call already pays.
  * A member's trait method, every handler's implementation of it, the
    fusion's forwarding impl and the generated host skeleton all render
    through the same member-parameter helper, so they cannot disagree.
  * [effect-args-hoisted] An argument that reborrows an implicit the *same*
    call passes is hoisted into a `let` first, or the two borrows overlap
    (`E0499`) — the same rule, and the same fix, as for a threaded effect
    value. A recursive call forwarding its own implicits is the usual way to
    hit it.
  * A bare call to an implicit parameter's name goes through the parameter,
    borrowing it for the call, which is why an argument of *that* call gets
    the same hoisting treatment.
* [effect-handler-generics] A generic handler is constructed **at** a type:
  `Plain::<i32>::new()`, from the type arguments the checker resolved for the
  `use` site. Written even where rustc could infer them — the emitter does
  not reason about the target's inference, and the case that needs them most
  (a handler with no constructor argument) leaves nothing to infer from.
  * A type parameter **no field mentions** gets a
    `__phantom_T: PhantomData<T>` field, initialized in `new`. A handler is a
    behaviour, and a generic one need hold nothing; Rust insists every
    parameter be used (`E0392`).
* [rs-fn-param-convention] A lambda passed into a fn-typed parameter binds
  its parameters the way the **callee's declared fn type** renders them,
  not the way the lambda's own annotation would: the callee fixes the
  calling convention, and its declaration is the only thing both sides can
  agree on. `f: (T) -> U` declares `FnMut(&T)` — a type variable is never
  known to be `Copy` — so an annotated `(n: Int) -> …` argument emits
  `|n: &i32|` and binds by reference.
  * The two sides used to decide `Copy`-ness on *different* types (the
    declaration on `T`, the lambda on its own `Int`), which disagreed
    precisely when the callee was generic: rustc rejected the call with
    `E0631`, and only when the lambda was annotated — an un-annotated one
    compiled, because rustc inferred the parameter from the bound. The
    convention is computed by one function mirroring the `Type::Fn` arm of
    `emit_type`.

## Effects [rs-effects]

* [effect-decl] Effects emit as Rust `pub trait`s whose methods take
  `&mut self` (handlers are stateful). The throw effect is the exception:
  it emits nothing, since it has no handlers [rs-throw-controlflow].
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
  An `intrinsic handler`'s member bodies come from
  `intrinsics::handler_member` instead, same shape. A ctor param that is a
  *dependency* is neither a field nor a `new` parameter, and the trait
  impl is replaced by a generated one — see [rs-effect-fusion].
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
  Kotlin backend (see COMPLETED.md "Emitter effect-environment fallback"
  under architectural facts).

### Effect dependencies via handler fusion [rs-effect-fusion]

**Implemented 2026-09-04; reshaped to the Has-accessor design 2026-09-14**
(user decision, FILE_SYSTEM.md §5.8.1 — adopted for both backends and
sequenced before the filesystem work; the single-effect case fuses too, by
the user's consistency call). A handler may declare a dependency
([effect-handler-deps]); Rust renders it by *fusing* the handlers of a
scope into one value. The original strategy decision was "B9, rebuilding,
no facets in Rust" (2026-09-03/04); COMPLETED.md's E1a holds the decision
log and the rejected alternatives. Every shape below was verified by
compiling and running it with `rustc` before the emitter was taught to
produce it (2026-09-14 for the Has shapes, including two instances of a
generic effect inherited through one `dyn` provider, and supertrait
elaboration carrying UFCS through it).

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

**The Has-accessor trait, beside every effect.** In fusion mode
`emit_effect` emits a second trait next to each effect trait:

```rust
pub trait __Has_Random<T> {
    fn __get_Random(&mut self) -> &mut dyn Random<T>;
}
```

Generic exactly as the effect is, so one declaration serves every
instance, and declared in the effect's own file so its identity crosses
modules through the same globs the effect's does [rs-imports] — no shared
definitions file. Fused values implement `__Has_E` per effect in scope,
**never the effect traits themselves**: member names cannot collide on a
fused value (a future `Fs` and `Net` both wanting `close` was the
motivating case), and two instances of a generic effect disambiguate with
the *Has* trait's turbofish rather than by name mangling.

**One fused parameter per fn, one effect included** (uniformity, user
decision 2026-09-14):

```rust
pub fn banner<__Fx: __Has_Console + __Has_Logger>(__fx: &mut __Fx) { … }
```

Generic rather than `dyn` for the same reason as before, now one step
removed: a fn forwards its fused value to a callee needing a **subset**
of its effects (`shout(&mut *__fx)`), and a Sized generic satisfies any
subset of its bounds by monomorphization, where `dyn`-to-`dyn` would need
upcasting that cannot reach a smaller conjunction. The cost is
monomorphization per fusion type; it stays finite because fusion structs
are not generic in their provider (see below).

**Member dispatch is accessor-then-method**:
`__Has_Random::<i32>::__get_Random(&mut *__fx).next_random(…)` — the UFCS
turbofish on the Has trait picks the instance, and the member call itself
is on `&mut dyn Random<i32>`, which is never ambiguous.

**One fusion struct per `use` site**, chained to whatever provided the
effects already in scope:

```rust
pub struct __Fx_main_3<'a, __H> {
    __outer: &'a mut dyn __Has_Console,  // or `dyn __Prov_…` for two or more
    __h: __H,                            // the handler this `use` registers
}
```

* **One `__outer` field**, not one per inherited effect: N reborrows of
  the same provider would alias. That single field is the only place a
  fused value needs a *nameable* type, and therefore the only reason
  provider traits exist:
  `pub trait __Prov_A_B: __Has_A + __Has_B {} impl<T: __Has_A + __Has_B + ?Sized> __Prov_A_B for T {}`
  — a conjunction of **Has** traits, emitted per file that needs one,
  where the blanket impl makes duplication harmless (a fusion built in
  module `M` satisfies `N::__Prov_A_B` too).
* The fusion's impls are **Has-accessor impls**: an inherited effect
  forwards through the provider —
  `__Has_A::__get_A(&mut *self.__outer)` — which type-checks because
  supertrait elaboration makes `dyn __Prov_…: __Has_A` hold, and the
  turbofish keeps two instances of one generic effect apart. An
  *independent* new handler's accessor returns `&mut self.__h`
  (bound `__H: Effect`); a *dependent* one returns `self`, because the
  raw effect impl lives on the fusion (below).
* **`dyn` in `__outer`** keeps monomorphization finite: a recursive fn
  that registers a handler and recurses maps its fusion type to itself
  instead of nesting `__Fx<__Fx<…>>` forever.
* **Generic over the handler** (`__H`), so a *generic* handler needs no
  re-derivation of its type arguments: `CyclicRandom::new(vec![…])` infers
  them, and the fusion's impls bound `__H` by what they need.
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
    fn log<__Fx: __Has_Console>(&mut self, __fx: &mut __Fx, message: &String);
}
impl __Impl_ConsoleLogger for ConsoleLogger { /* the written body */ }
```

`&mut self` is kept, so `self.state` still works, and the trait is only
ever a *bound* — never `dyn` — so its methods may be generic.
Dependencies travel **uniformly** as a Has-bounded Sized generic, one
dependency included (user decision 2026-09-14: consistency over a special
case). The Sized value is built by a per-handler adapter over the one
provider field, implementing each dependency's Has trait by forwarding:

```rust
pub struct __Deps_CountingAudit<'a, __P: ?Sized> { pub __p: &'a mut __P }
impl<'a, __P: __Has_Console + ?Sized> __Has_Console for __Deps_CountingAudit<'a, __P> { … }
```

(`pub __p`, because the adapter is declared beside its handler but
constructed inside fusion impls in whichever module `use`s it.) The
fusion's *raw effect impl* for the dependent handler is where the trick
lands — `&mut self` is destructured into **disjoint field borrows**
first, so the handler's state and its dependency are two separate
`&mut`:

```rust
impl<'a, __H: __Impl_ConsoleLogger> Logger for __Fx_main_3<'a, __H> {
    fn log(&mut self, message: &String) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps_ConsoleLogger{ __p: &mut **__outer };
        __Impl_ConsoleLogger::log(__h, &mut __deps, message)
    }
}
```

The dependent handler's Has-accessor then returns `self`: the fusion is
the `dyn Effect` its own accessor hands out. Independent handlers keep
today's `impl Effect for H`, and their accessor returns `&mut self.__h`.

**The dyn boundaries.** Two ABIs cannot take a generic fused parameter,
and each opens by rebuilding a Sized fused value:

* **Platform `main`** keeps one `&mut dyn E` parameter per platform
  effect — the host constructs one implementation each and calls
  `salvo_main` with them [rs-platform-entry] — and its body begins with a
  generated combiner (`__Dyn_main_1<'a>`, one dyn field per effect, a
  Has impl each) that the rest of the body threads.
* **Fn values** declare **one** `&mut dyn` provider parameter for their
  whole effect list — the single effect's Has trait, or a `__Prov_…`
  conjunction — since two separate reborrows of the caller's one fused
  value would alias (`E0499`). A lambda's body opens with a combiner
  over that provider (`__FxDyn_demo_1<'a>`); a named fn passed as a
  value gets the same combiner inside its adapter closure. The caller
  threads its fused value into the position by plain unsizing (the
  blanket impl makes any fused value a provider).

**Arguments that reach the fused value are hoisted** into a temporary:

```rust
{ let __a1 = &(format!("drew {} at {}", Counter::total(&mut __fx2))); println(&mut __fx2, __a1) }
```

`fx.a(&fx.b())` is two overlapping `&mut` (`E0499`). With per-effect
parameters the two receivers were disjoint variables, so this hazard is new
with the fusion — and it is why the emitter renders such calls as block
expressions.

**No facets in Rust** (user decision 2026-09-04): each backend leverages
its own language. Rust has no erasure, so one generic `__Has_Random<T>`
declaration serves `Random<i32>` and `Random<String>` alike and
multi-instance generic effects need no mangling. Kotlin's Has interfaces
are per *instance* instead ([kt-effect-fusion]) — which is also what
retired the facet design there.

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
  parameter. Kotlin accepts *this* one (erasure), so it is a real
  divergence, and the way to lift it is to derive the handler's arguments at
  the `use` site by unifying its `of` clause against the checker's instance.
* A `use` whose **effect instance is still generic** — an effect list generic
  in the enclosing fn (`fn f<T>() [Random<T>, use]`). The fusion names its
  effects in impl headers, so an unresolved `T` would be an undeclared type.
  Lifting this means threading the enclosing fn's generics into the generated
  items.
  * `use Relay<Int>()` **is no longer an example**: the written type
    arguments now bind the handler's generics [effect-handler-generics], so
    that instance is concrete and the cut does not apply. Until 2026-09-06
    they were discarded, and because this report fires only on the *fusion*
    path a single-effect program emitted invalid Rust instead (`E0283`, plus
    `E0392`).
* A generated **provider or accessor trait name claimed by two different
  effect sets**: sanitizing `<`/`,` to `_` is not injective, so an effect
  literally named `Random_i32` collides with `Random<i32>`. Vanishingly
  unlikely, but silently reusing the wrong trait would be wrong code.

Both are reported at the `use`/handler that causes them, never mis-emitted.

## Functions and calls

* [fn-dot] Dot-notation calls resolve to a declared fn or effect
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
* [intrinsic-fn] Every std lowering lives in this crate's `intrinsics.rs`
  (`fn_call`), keyed by the checker-resolved declaration (name + first
  parameter's base type name, so `size(Str)` / `size(List<T>)` /
  `size(T[])` are three entries). `copy` [rs-copy] and `discard` are the
  exceptions: they dispatch on the argument's own shape, so
  `emit_intrinsic_call` handles them before consulting the table. An
  intrinsic with no entry is a codegen error naming it.
  * The argument boundary keeps the place/owned distinction [rs-borrows]:
    a place splices raw so a method-style lowering (`list.push(..)`)
    borrows natively, while a variadic tail splices owned because it lands
    inside `vec![..]`. Backwards, this either double-clones or moves out
    of a borrow. A parameter the lowering mutates in place therefore takes
    the raw place; everything else its owned rendering.
  * Rust does not spell type arguments out the way Kotlin must: `vec![]`
    stays `vec![]`, since [call-type-args] guarantees the element type is
    either written or annotated, and both reach rustc through the rendered
    `let` annotation or parameter type. A lowering *is* handed the call's
    resolved type arguments for the rare construct that needs them spelled
    out (`Vec::<T>::new()`), but the current table ignores them.
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
* [rs-platform-host] [platform-tree] [cli-platform] The host file for module
  `M` is `platform/<M>.rs`, mounted from the crate root as
  `#[path = "platform/<M>.rs"] pub mod platform_<M>;` — the module's own mod
  name with a `platform_` prefix, so a host and the module it implements for
  can never collide [rs-crate].
  * Rust requires `fn main` in the crate root, and the generated entry point
    there is `salvo_main`, so the crate root gains a delegation:
    `fn main() { crate::platform_<M>::main() }`. The `rustc` invocation is
    therefore unchanged, and `entry_hint` still names the crate root.
  * The generated skeleton is `pub struct <E>Host;` plus
    `impl <path>::<E> for <E>Host` with every member stubbed
    `todo!("implement <E>.<member>")`, followed (in the entry module) by
    `pub fn main() { <path>::salvo_main(&mut <E>Host, …) }`. Member
    signatures come from `emit_member_param_list`/`emit_return_type` — the
    same renderers `emit_effect` uses.
  * Every reference is a fully qualified `crate::…` path rather than an
    import: the host is a mounted module, and `<path>` is `crate` for the
    crate-root module and `crate::<mod_name>` otherwise. A host struct
    belonging to another module's host file is reached as
    `crate::platform_<N>::<E>Host`. `module_mod_names` is shared with
    `emit_program` so the skeleton and the mounting cannot disagree on a
    name.
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
* [once-fn] `once` fn parameters emit `impl FnOnce(…)`; consuming
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

* [rs-fn-mangling] [qual-overload] The same collision, one level up: two
  qualifiers may share a name over different subject types, and since
  qualifiers are erased [qual-erasure] both would emit `Q_qualifies`. The
  subject's base name disambiguates — `Filled__List_qualifies` — and only when
  the name is actually overloaded, so a program with one `Filled` emits the
  plain `Filled_qualifies` it always did. The predicate call site resolves the
  same declaration from the subject in hand, so the two agree by construction.
  Kotlin needs no equivalent: the JVM overloads on the parameter type.

## Runtime modules [rs-runtime-source]

* [rs-runtime-source] Code the backend *ships* rather than generates lives in
  `runtime/*.rs`, included verbatim (`include_str!`) and written into a
  program's output only when it touches the feature. They are real source
  files, not string literals inside `emit.rs`, and
  `tests/runtime_tests.rs` compiles each one directly with `rustc`: a syntax
  error in one is then a failure in *that* module rather than in some
  unrelated end-to-end test, and a change to one reviews as code instead of
  as a diff of an escaped string.
  * Each must compile **warning-free as a library crate** — it is spliced
    into user output, where a warning is noise the user cannot fix.
  * The test's list of modules is what makes it complete rather than a
    sample, so a new runtime module belongs there the moment it exists.
* [rs-collections] The insertion-ordered `Set`/`Map` [col-insertion-order]
  are such a module: `runtime/collections.rs` defines `SalvoSet`/`SalvoMap`,
  because Rust's standard library has no ordered hash container (`HashMap`
  has no order; `BTreeMap` is *key* order and demands orderable keys). The
  representation is a slot vector plus a hash index — insertion order is the
  slot order, a removal tombstones its slot so the rest keep their
  positions, and the vector is compacted once the tombstones outgrow the
  live entries.
  * `Map<K, V>` is `SalvoMap`, `Set<T>` is `SalvoSet`; the sorted pair maps
    to `BTreeMap`/`BTreeSet` instead [col-sorted], which need no runtime.
  * **Trait bounds live on the impl blocks, not on the struct**, and only
    where they are needed: a generic Salvo fn mentioning `Set<T>` emits a
    signature carrying only the bounds *Salvo* knows about, so a bound on
    the type definition would make that signature unsatisfiable. `iter`,
    `len`, `keys`, `values` and the `Display`/`Debug` impls are unbounded;
    the hash operations require `Hash + Eq + Clone`.
  * The runtime is emitted only into a program whose modules mention one of
    the types or call one of their constructors, and the crate root mounts
    it with `mod collections;` [rs-crate].

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

Known acceptable divergences (documented, not errors): extra `.clone()`s
where Kotlin shares references; `Debug`/`Display` formatting of `Option`
values differs from Kotlin's `null` printing (the checker's narrowing rules
make user programs format only unwrapped values).
