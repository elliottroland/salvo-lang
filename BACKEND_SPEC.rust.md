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

## Assertions

* [assert-trap] [rs-assert-trap] A failed assertion is a **panic** whose message
  is Salvo's: `expr!` lowers to
  `.expect("salvo: value is absent at <module>:<line>:<col>")`, `assert!(c, m)`
  to `if !(c) { panic!("salvo: {} at …", m) }` and `unreachable!(m)` to the
  `panic!` alone — which types as `!` and so stands wherever a value is expected.
  The message composition sits *inside* the panic, so a written message is built
  only when the assertion fails.
  * `expect` rather than `unwrap_or_else(|| panic!(…))`: same text, no closure,
    and `Option::expect` panics with exactly the message given.
  * The location is the **module path**, not the file name — a file's display
    name depends on the loader, and emitted output must not.
* [rs-opt-borrow] `expr!` on an owned `Option` held by a **local** reads it
  *through a borrow*: `p.as_ref().expect(msg).clone()`, the same form a narrowed
  read takes (`narrow_unwrap`). Moving out of it (`p.expect(msg)`) is kept for
  the four cases where nothing reads it again: a Copy payload (the `Option` is
  Copy too), an `Option<&T>` operand, a `proj`-typed result (the borrow *is* the
  value), and a span the checker recorded as a move (`linear_moves`,
  `state_takes` — a clone there would duplicate an obligation, and a `Reply<T>`
  is not `Clone`). A *field* operand needed nothing: `emit_owned` already clones
  one. Before this (2026-09-23) two `!`s on one local were two moves, rustc
  E0382, while the checker allowed both — reading an optional is free.
* [rs-read-mode] The emitter carries a **wanted mode** while it walks an
  expression: `Read` (a `&T` is enough) or `Own` (a value is needed). `Read` is
  the default and `emit_owned` raises it for the duration of its walk, so a path
  that *can* answer with a borrow clones only where the position needs ownership
  — the copy `[copy-opt-in]` says should not happen without the program asking
  (2026-09-23, ROADMAP.md's "One read, one mode"). `emit_read` is the
  counterpart of `emit_expr` for a position that keeps what it is given.
  * Three sites consult it, and the two that read a `!` share one predicate
    (`owned_optional_local`) so they cannot disagree: the `NonNull` arm of the
    expression walk, and `borrowed_arg`, which needs neither the clone nor a
    second `&` because the unwrap already answers a reference. A **kept**
    parameter therefore receives `p.as_ref().expect(…)` and a **consuming** one
    `p.as_ref().expect(…).clone()`.
  * An argument to an **intrinsic** takes its mode from the *intrinsic's own
    declaration* rather than from the position the call sits in
    (`intrinsic_arg_code`, 2026-09-23): `Read` for a parameter the declaration
    keeps, `Own` for one it consumes (`=> !elem`), for one typed `Mut` (the
    lowering writes through it) and for the variadic tail (whose store clones
    [fn-variadic]). So `contains(str, needle) => str, needle` hands its template
    `trap.as_ref().expect(…)` where it used to clone. The templates tolerate the
    reference because they already receive one whenever the caller's variable is
    a `&T` binding — `{}.contains(&{}[..])` and `{}.chars().count()` are method
    calls and `&x[..]` indexes through a `&String`.
  * [rs-narrow-mut] `x!` in a **`Mut` intrinsic parameter** position reaches the
    payload mutably: `xs.as_mut().expect(msg)`. Rendered owned — which is what
    the pre-2026-09-23 path did — `add(xs!, 3)` emitted
    `xs.as_ref().expect(…).clone().push(3)`, which compiles, appends to the
    clone, and printed `1` where Kotlin printed `2`: the last shape of the
    narrowed-`Mut` defect closed 2026-09-20, reached through `!` instead of
    through a narrowing.
  * A **narrowed** read obeys the mode the same way (`narrow_unwrap`,
    2026-09-23): `name.as_ref().unwrap()` / `o.u1()` under `Read`, with
    `.clone()` added under `Own`. A **Copy** payload is a third thing — copied
    out of the representation (`*o.u1()`, `n.unwrap()`), free, and a value in
    both modes. The mutable read is `narrow_unwrap_mut`, unchanged.
    * The predicate a `&T` position asks first is `narrowed_borrow`: a narrowed
      non-Copy read *is* the reference, so `borrowed_arg` returns it as it
      stands rather than borrowing a clone back
      (`len_of(name.as_ref().unwrap())`, `len_of(b.label.as_ref().unwrap())`).
    * Two positions walk with `emit_place` and so run under the *ambient* mode,
      which at a statement is `Read`: a **move-mode binding**
      (`emit_bound_value`) and a **consuming `for` subject**. Both are owned, and
      both raise the mode explicitly. Without that, `let s: InStream = opened`
      under a narrowing bound a `&InStream` — fifteen rustc E0308s across
      `examples/files` alone, which is the reassuring half of this mode's failure
      mode: dropping a clone that was load-bearing does not compile, so rustc
      is the safety net rather than the output being quietly wrong.
  * Still owned, and recorded in ROADMAP.md: an **interpolated** value. The
    native case would take the reference happily (`format!("{}", &String)`), but
    the `to_str` cases in the same function write `to_str(&{arg})` and
    `{place}.field`, so the site would have to ask the predicate three times —
    which is the argument for a rendering that *reports* what it produced
    (`Rendered { code, is_ref }`).

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

* [safe-call] [rs-safe-call] `receiver?.member` emits
  `if recv.is_some() { Some(<inner>) } else { None }`, with the inner access
  reading the narrowed payload through the existing unwrap [rs-option]. The
  `Some(..)` is **omitted** when the member is already optional, or the result
  would be an `Option<Option<T>>` the declared type does not have — an E0308
  rustc caught, which Kotlin never saw, having no wrapper to double.
* [elvis] [rs-elvis] `subject ?: rhs` lowers to
  `match <subject> { Some(__v) => __v, None => <rhs> }`. A `match` rather than
  `unwrap_or_else` because the right side may be an **escape**: a `return`
  inside a closure returns from the closure [expr-escape]. The `match` is also
  what gives the single evaluation.
* [type-basic] Internal types map natively: `Str`→`String`, `Int`→`i32`,
  `Long`→`i64`, `Float`→`f32`, `Double`→`f64`, `Bool`→`bool`,
  `Char`→`char`, `Byte`→`u8`, `Never`→`!` (the language docs' original
  `u64` for `Long` was a spec bug — `Long` is signed; fixed during M8).
  `Any` has no Rust mapping yet: referencing it is a codegen error
  ([backend-never-wrong]).
  * [byte-value] [bytes-type] `Byte`→`u8`, and **`Bytes` and `Mut Bytes` are
    both `Vec<u8>`**: unboxed, no runtime class, and `Mut` erasing as it does
    for every other type here [type-canbe-mut], so dropping it renders
    nothing. The buffer class Kotlin has to ship ([kt-bytes]) is simply what
    this backend gets from `Vec`, `slice` included (a `to_vec()` of a range).
    `to_byte`/`to_int` are `as` casts with the **source type named**
    (`(((x) as i32) as u8)`): rustc infers an unsuffixed literal's type
    from the cast, so `(-1) as u8` would make the literal a `u8` and be
    rejected instead of meaning 255.
* [op-promote] Rust has no mixed-width operators (`i32 + i64` is E0277),
  so a checker-recorded promotion casts the operand **as a whole**:
  `((n * 2) as i64)` — the inner parentheses matter, since `as` binds
  tighter than every arithmetic operator and `(n * 2 as i64)` would cast
  only the `2`. Targets are `i64` and `f64` only (widening goes up within
  a class).
* [lit-adopt] An adopted literal renders at its **checked** type
  (`1i64`, `3f64`, `0.5f32`); an unsuffixed literal at its default type
  stays bare for inference [lit-numeric]. [op-convert] lowers to `as`
  casts, whose semantics match Kotlin's `toX()` pairwise (saturating
  float→int, low-32-bits `i64`→`i32`).
* [effect-at] Erased like the scope selector below: the checker records
  the resolved effect per call (`effect_calls`), and emission is the
  ordinary member dispatch.
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
* [rs-seq] std's sequence functions [seq-iterator]: the `List` fast paths
  are std functions (2026-10-04, [platform-value-type]): `map` and `reduce`
  are Salvo loops, and `filter` is a platform fn (`platform/core/seq.rs`)
  cloning each kept element. Their fn-typed parameters render as
  `&mut impl FnMut(&T)` ([rs-fn-param-convention]; `&mut dyn FnMut` for a
  platform wrapper), which is what gives a lambda argument its parameter
  types: a Rust closure bound to a `let` or nested in another closure's
  argument cannot infer them. A **named fn** argument wraps in the adapter
  closure [fn-contract], since a fn item's own convention is by value
  (E0631 against `FnMut(&T)`). `runtime/seq.rs` is left holding
  `salvo_pair_mut` [rs-elem-mut].
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
    the result [iter-mint]; the intrinsic containers (`Vec`, arrays, `String`)
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
    stopped writing such an iterator when the lazy pair was removed (2026-09-10);
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
    (`move |__p: &mut __Iter_X| next(__p)`), and its parameter
    is annotated because rustc cannot infer it through the `&mut dyn FnMut`
    coercion an unkept position renders.
* [rs-none-unit] `None` is Rust's `()`, and a fn returning it has no return
  type — so `return None` emits a **bare** `return`. The test is the *fn's*
  rendered return type, not the value's: `return None` in an
  `Option`-returning fn is `return None;` and correct. The literal is dropped
  rather than evaluated; any other `None`-typed value runs for its effects
  first.
* [type-tuple] Tuples map to native Rust tuples (any size).
* [rs-lazy-adaptor] [seq-lazy] **A struct over an iterator** (2026-10-03):
  a struct's type parameters carry `Clone` but not `'static`, since a lazy
  adaptor is instantiated at a pass that borrows its source
  (`Taking<ListYield<'s, T>, &T>`); a fn field's callback is `'static` by its
  own bound, which is all the struct needs. A fn that hands back such a struct
  takes its callbacks owned as `impl Fn(…) + Send + Sync + 'static` (the
  `Arc<dyn Fn + Send + Sync>` field's bounds), and that includes the fns an
  implicit group spreads (`?Yield<It, T>`'s `next`), so `owns_callbacks`
  counts a fn with implicit groups too.
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
    source *and* its callback) builds on both targets, so composing iterators
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

The central design (per docs/language/Deductions-and-Ownership.md): the checker's deduction
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
  type. Structs additionally `#[derive(Clone, Debug, PartialEq)]` — `PartialEq`
  unconditionally, since it costs nothing where Salvo refuses `==` anyway
  [col-equality] — plus `Eq` and `Hash` for a struct that *has* a `hash`, and
  `Eq, PartialOrd, Ord` for one that has a `cmp` (stamped or hand-written
  [obligation-by]), **when every field derives**: the interim structural
  identity of a `List<Point>` or `(Int, Point)` key reaches the element's
  derive rather than its Salvo fn [col-hashed-ordered], so the derive is what a
  container over it stands on. A struct with a fn-typed field derives only `Clone`:
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
  `'a` is generated onto **every** source parameter (`proj(a, b)`)
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
  * [elvis] The same rule for the **`?:` read** and for an `is` binding over
    an optional borrow — a `get(xs, i)` hoisted into `__pickN`, or an
    `OptRef` local: `*__pick.unwrap()` for a Copy scalar, `__pick.unwrap()`
    kept as the borrow when the picked type is `proj`, `.unwrap().clone()`
    otherwise (`subject_is_optional_borrow`, `emit_narrowed_read`). Found
    2026-09-27: `get(xs, 0) ?: 10` emitted `&i32` against `10`, on lists and
    maps alike; the `proj` case had compiled only because `.as_ref()
    .unwrap().clone()` over `Option<&T>` happens to yield `&T`.
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
  * An intrinsic argument whose parameter the declaration **consumes**
    (`=> !value` — `send`, `discard`, `add`, `insert_sorted_by`, `put`,
    `reduce`'s seed) renders **owned**, not as a place: the lowering takes ownership, so
    the same rendering an ordinary consuming call gets applies (a real partial
    move stays a move [fate-move-mode], a read the caller keeps clones).
    Without it the intrinsic path was the *only* consuming position emitting a
    bare place into a moving one, which rustc reported as E0507 with no Salvo
    diagnostic (`send(out, last)`, `add(xs, last)` on a handler's own
    non-Copy field; fixed 2026-09-15, with the checker rule that refuses that
    shape outright — [effect-state-store]'s read direction).
* [rs-proj-struct] A struct with a `proj` field — or an owned field whose
  type has one, transitively — is a **borrowing struct**: `struct
  ListYield<'s, T> { items: &'s Vec<T>, at: i32 }`, with `<'s>` on owned
  view-typed fields; every mention elides (`ListYield<'_, T>`); a struct
  literal borrows into its `proj` fields (`&list`); it is returned *by
  value* (the struct carries the lifetime, no `&` wraps it).
* [rs-proj-lends] The lifetime a view carries reaches the parameters it
  borrows [proj-infer]: with one reference parameter elision ties them;
  with more, `'a` is named on every lent parameter (`Checked::fn_lends`)
  and the return. A lent parameter that is **itself a borrowing struct**
  defeats elision even alone — `p: &mut ListEnumYield<'_, T>` has two
  input lifetimes — so `'a` is named there too, and it tags the struct's
  **inner** (source) lifetime, never the `&mut`: the returned view borrows
  the iterator's *source*, so the reborrow of `p` stays free for the next
  turn, exactly as [rs-proj-struct] ties a derived return (added
  2026-09-23 for `Enumerated<T>`, the first view struct a `next`
  answers). A **lent implicit position** (`?iter: (c: C) -> Mut It holds proj(c)`) renders `&'c C` under a lifetime `'c`
  named on the enclosing fn's kept parameter `c` — the result's type
  (`It`) is fixed at the call site, so the borrow it holds cannot be a
  fresh per-call one; the enclosing fn must keep `c` (a consumed one has
  nothing a view could outlive — reported). Re-pointing entries
  (`v.items: proj(other)`) tie `'r` on the target struct and the
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
* [rs-elem-mut] [proj-mut] **Mutable element handles** (P-3 + P-9, user
  decisions 2026-09-24) have two renderings, and neither is a bound `&mut`:
  * A **statement-scoped** handle — `bump(get(es, i)!)` in a `&mut`
    position — splices the mut lowering directly:
    `bump(es.get_mut((i) as usize).expect("salvo: value is absent at …"))`.
    The `&mut` lives exactly as long as the call, so no exclusivity window
    opens.
  * A **bound** handle (its bind event in `Checked::handle_muts`) is
    **virtual**: `let __hN = (i) as usize;` plus a presence check at the
    mint (where `!` traps, matching Kotlin's `!!` timing —
    `es.get(__hN).expect(…)`), and the binding's every use re-materializes
    the place (`es[__hN].n = …`, `bump(&mut es[__hN])`, `es[__hN].clone()`
    in owned positions) — `BindKind::ElemMut` + `elem_places`. Deliberately
    not a bound `&mut`: Salvo's poison discipline permits reads of the
    container between uses of the handle, which a live `&mut` binding would
    make E0502 — the same alignment argument as [rs-borrow-locals], resolved
    the other way.
  * A kept parameter whose **elements** carry `Mut` (`List<Mut T>`,
    `Mut T[]` — `type_has_elem_mut`) renders `&mut Vec<T>`: the container
    lends mutable handles, so the write must reach the caller's storage
    through it even though no structural mutation is permitted.
  * A **proven-distinct pair in one call** ([elem-distinct],
    `Checked::distinct_pairs`) renders as a `salvo_pair_mut` preamble —
    `let (__pm0, __pm1) = salvo_pair_mut(&mut es[..], i, j).expect(…);`,
    one `split_at_mut` [rs-runtime-source], `i != j` checker-guaranteed —
    and the call takes the two `&mut` halves. The `.expect` keeps the
    message and timing of a single handle's `!`. Statement-position calls
    only (the v1 cut): a pair call in a value position is a reported
    codegen error naming the remedy, as is a pair argument that is neither
    a direct `get(place, i)!` mint nor a bound handle.
  * **The v1 cut** [backend-never-wrong]: a bound mutable handle minted from
    anything but a direct `get(place, i)!` over a pure place (the total
    Idx-claimed `get`, `first`, a call-result container) is a reported
    codegen error naming the remedy. Kotlin needs none of this — objects
    alias natively, so the handle is the element reference.
* [rs-cmp-deref] **A borrowed Copy scalar is copied out in a comparison**
  (2026-09-25): Rust implements `&i32 + i32` but not `&i32 == i32`
  (E0277) — and `&i32 < i32` likewise — so a lending call's scalar result
  (the total `get` at an `Idx`/`KeyOf` claim, `first(NonEmpty)`) is
  dereferenced in a comparison operand and **nowhere else**: arithmetic,
  interpolation, `!`-unwrapped optionals and `for` elements already render
  correctly, and non-Copy operands must not be touched (a deref there would
  move out of a borrow). Keyed on the checker's derived-call table plus a
  Copy-scalar type, so it fires exactly on the shape that breaks. Kotlin has
  no references and needs nothing. The defect this closes was found writing
  `expect(get(m, k) == 1, …)`, which `core.map`'s annex now spells that way
  deliberately.
* [rs-loc] **Locator-specialized lending** (④a slice 1, 2026-09-24 —
  re-founding step ③'s mode-specialization on the locator model,
  ROADMAP.md's "Recorded refinements"): a named lending fn whose
  result some call site uses mutably gets a **demand-driven locator
  variant**, `{name}__loc`, beside the read emission.
  * The variant answers **position data** — `usize` for a total element
    lend, `Option<usize>` for an optional one, optional exactly where the
    read emission was, so `!` keeps its message and timing. Its lent
    parameters drop to *read* mode (the search borrows nothing mutably),
    and it carries **no lifetimes** — a locator is owned data, which is
    what lets later slices pass it through closures and traits.
  * The **use site materializes** the handle, statement-scoped:
    `{ let __l = callee__loc(&anchor, …).expect(…); &mut anchor[__l] }` —
    the read borrow over before the write borrow begins, per-statement
    `noalias` kept. The anchor must be a plain place (a call-result
    container has no storage to re-index); anything else is a reported
    error naming the remedy. The direct `get(place, i)` shape
    short-circuits to its inline splice (the degenerate locator).
  * Body transform: return-path forwards take their callees' `__loc`
    variants (demand closes transitively, `lend_mut_demand`); a
    derived-return intrinsic takes its **locator form** (`get` answers
    `Some(i)` under a presence test, `None` where the read answered
    `None`); an intrinsic without one, or a return shape beyond the plain
    and optional element lend, is a reported error, never a silent read
    lowering [backend-never-wrong].
  * The read emission stays the default for read uses: Salvo's read
    handles are shared, and shared renders `&` — the `Mut` in a return
    type is *permission*, mode is *use* ([fate-move-mode]'s split).
  * **Bound handles** (④a slice 2): any locator-expressible lending call
    mints one — the captured locator re-materializes `anchor[__hN]` per
    use, so container *reads* between uses stay legal (a bound `&mut`
    would be E0502). ①'s "direct `get` only" cut is retired.
  * **Fn values** (slice 3): a fn type whose return is a wholesale
    mutable lend renders as a **locator closure** —
    `impl FnMut(&C, &L) -> Option<usize>`, read-mode parameters — and a
    lambda filling such a position emits in locator mode. This is what
    lets a mutable handle cross a closure boundary at all; a
    `&mut`-returning `FnMut` would tie the borrow to the closure. The
    `?at`/`Locate` idiom [col-locate] rides it, with implicit positions
    rendered the same way (`implicit_param_type_borrowing`).
  * **Effect members** (slice 4): a member whose return is a wholesale
    mutable lend carries **both faces** — the read one, explicitly
    lifetime-tagged (`fn lease<'a>(&mut self, es: &'a Vec<T>) ->
    Option<&'a T>`, since elision with `&mut self` present would tie the
    borrow to `self`), and `{member}__loc`. Declaration-driven: the
    trait, every handler impl and the monitor adapter all carry both, and
    a `Mut` position routes to the locator face. Such an effect has **no
    lock adapter**: a borrow cannot escape a mutex guard, so a
    mutable-lending effect is local by nature.
  * **Search loops** (slice 5): inside a locator variant, a `for`
    directly over a list lowers to an *indexed* loop and the element
    binding becomes a captured-index handle, so `return e` answers the
    found **position**. That is the pass-hidden-position and NLL-loop
    case, both lifted.
  * **Covered positions** ([canbe-entry], rung ④b): a callee whose clause
    declares `canbe` coverage renders its covered parameters as **one
    shared anchor plus a `usize` locator each** —
    `fn attack(__anchor: &mut Vec<Entity>, __c0: usize, __c1: usize)` —
    and materializes `__anchor[__cN]` per statement inside the body. The
    call site passes `&mut container` once and the positions after it. Two
    `&mut` into one container cannot coexist, which is why coverage
    changes the *representation* rather than relaxing a check; aliasing is
    then exact (one storage), so a covered call behaves identically to
    Kotlin's native aliasing — including the case where both handles are
    the same element. Reported, loudly: a covered argument that is not an
    element handle of a bound container, and covered positions naming
    *different* containers (they share no anchor).
    * **An anchored entry's anchor is a parameter** (`=> t canbe in
      lib.tracks` — the path is rooted at one), so the callee keeps that
      parameter and the covered positions add nothing but their index:
      `fn trade(squad: &mut Squad, __c1: usize, __c2: usize)`, indexing
      `squad.members[__cN]`. Only the plain `a canbe d` form, whose anchor
      no parameter names, grows an `__anchor` of its own. Synthesizing one
      *beside* the parameter is what made every anchored call E0499
      (`trade(&mut squad, &mut squad.members, …)`) — the form was
      documented and could not run on this backend at all until
      2026-09-25. The anchored form is therefore also what licenses
      **handles passed beside their own container** in one call: the pair
      shares the anchor rather than borrowing it twice. The call site
      checks the agreement it rests on — a handle of a *different*
      container than the clause anchors it in is reported, naming both.
  * **What remains cut, loud**: a lend whose anchor is not a plain place
    of a known indexable type (a bare generic container has no index —
    the recorded lift is the type-erased locator, COMPLETED.md's log's
    second GB-5 addendum), and branch-dependent path sets (generated
    path enums, when a case first needs one). Kotlin: nothing — objects
    alias; parity pinned by the e2e cases.
* Views of temporaries are refused by the checker [proj-anywhere]; the
  only thing this backend adds is that rustc would have said the same
  (E0716).
* [rs-float-text] **A float's text comes from the runtime helper, not
  `Display`** (built 2026-09-25, closing the parity defect of 2026-09-14).
  Salvo's rule is Kotlin's [interp-float], which Rust's `Display` matches in
  neither respect: it writes the number out in full (`100000000000000000000`
  for `1.0E20`) and drops the `.0` (`2` for `2.0`). `strings::salvo_f64_text`
  and `salvo_f32_text` rearrange `{:e}`'s output — the same shortest
  round-tripping digits Kotlin's `toString` chooses, so only the arrangement
  differs — into the plain window or the scientific form, and answer `NaN` /
  `Infinity` / `-Infinity` for the specials. They take anything that borrows
  the float (`Borrow<f64>`), so no call site writes a deref.
  * Verified against Kotlin's own output on 33 values, including both
    threshold boundaries, the denormal minimum, `MAX`, `1e300`, `1e-300` and
    the f32 cases.
  * **Three sites**, because the rule has to hold wherever a float becomes
    text: an interpolated part, `to_str` of a value holding floats, and a
    struct field rendered field-wise [interp-struct]. A container is rendered
    **element-wise** here rather than by the runtime's `Display` impls, which
    can only call `Display` on their elements; the renderer recurses, so
    `List<List<Double>>` and a map's value half are covered. A type holding no
    float keeps its existing rendering, so nothing else in the output moved.
* [rs-loop-temp] **A `for` over a temporary hoists it** (built 2026-09-25,
  closing a defect open since 2026-09-18). An iterator-driving loop *binds* the iterator
  (`let mut __loopN_pass = …`), so a Rust temporary the subject borrows dies at
  the end of the statement it was written in (E0716) — while the language allows
  the shape deliberately ("a view of a temporary may be *used* within its
  statement", [proj-anywhere], and a `for` is that use) and Kotlin's reference
  needs nothing. So the subject's temporaries are bound to locals in front of
  the loop, which is rustc's own suggestion:
  `let __t1 = vec![1, 2]; let mut __loop1_pass = iter__3(&__t1);`.
  * **Which temporaries**: the subject's calls are walked innermost-first and
    every argument that is **borrowed and not a place** is hoisted, registered
    in the same substitution table [rs-mut-arg-hoist] uses. An owned (consumed)
    position needs nothing — a moved value is not borrowed from anywhere. So a
    view of a temporary hoists through the call that holds it
    (`filter(iter(list_of(…)), …)`), while a native container loop needs no
    hoist at all (the temporary lives to the end of the `for` statement there,
    which includes the body).
  * check.rs's [proj-anywhere] comment used to claim the temporary "lives for
    the whole loop statement on both backends", which was simply false; the
    comment now points here.
* [rs-mut-arg-hoist] **A read before a mutation, in one expression** (built
  2026-09-25, closing the defect of the same day). Rust holds a read borrow
  for the whole expression it sits in, so a sibling that mutably borrows the
  same place collides with it — `format!("{} {}", b.n, bumped(&mut b))` and
  `label(&b.tag, bumped(&mut b))` are both E0502 — while the language only
  orders the two ([deduce-same-call]: arguments are evaluated left to right,
  and a read is not a consumption), and Kotlin runs them. So the *read* is
  hoisted into a `let` in front of the expression, which is the order the
  language already gives it:
  `{ let __r1 = b.n; format!("{} {}", __r1, bumped(&mut b)) }`.
  * **Which siblings**: a call's arguments and an interpolation's parts —
    the two positions where a rendering holds a borrow. A sibling is a
    hoist candidate when it *is* a place and the position borrows it (a
    `Ref`/`RefMut` parameter mode; in `format!` a Copy scalar, since
    anything else is already owned by a `.clone()` or a `to_str`), and it
    is hoisted only when a **later** sibling mutably borrows an overlapping
    place — the callee's parameter modes say which arguments those are, for
    the nested calls too. Overlap is prefix-wise over place paths, with a
    subscript stopping the path at its container [fate-field-disjoint], so
    a read of `e.rings` beside a mutation of `e.hp` is left alone.
  * **What it will not do**: a read of **mutable data** cannot be copied out
    of the way, because a snapshot on Rust against a live handle on Kotlin
    is exactly the divergence the rule exists to prevent. That shape is
    reported instead, naming the two things the program can say instead —
    `copy(place)` for the snapshot, or the mutating call in a statement of
    its own [backend-never-wrong].
  * **Cut**: the modes come from the callee the checker resolved, so a call
    through an **effect member or a fn value** plans nothing and keeps
    whatever rustc makes of it (the state before this rule, not a new
    divergence). Covered and proven-pair calls plan nothing either — they
    render their own positions and preamble, and leave no read borrow
    standing.

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
  `Ui` variants (positional over the runtime union's non-`None` arms,
  qualifiers erased; literals collapse [type-literal]).
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
  * **Two further sites, added 2026-09-20** (the rule shipped with the first
    two on 2026-09-10 and the others were silently wrong until then; the
    repro is in COMPLETED.md):
    * **An `intrinsic` parameter the declaration types `Mut`.** The
      intrinsic path renders arguments itself, and rendered every place with
      the read form regardless of mode, so `add(a, 9)` on a narrowed
      `Mut List<Int>?` emitted `a.as_ref().unwrap().clone().push(9)` — and
      because the nullable read clones *per read*, the element was gone on
      the next line. The place now takes the mutable unwrap when the
      parameter is `Mut` and is not consumed, falling back to the read form
      when the place carries no narrowing (so an ordinary `Mut` argument is
      unchanged).
    * **The `^` branch shadow, when the peeled payload is `Mut`**
      ([rs-widen-shadow]): bound as `let x = <mutable unwrap>` with
      `BindKind::RefMut` rather than an owned clone, so a mutation inside the
      branch reaches the storage and survives the branch. No `mut` on the
      binder — it is a reference, and the generated code stays warning-free.
      Reads are unaffected: a `&mut T` binding is what an ordinary `Mut`
      parameter already is.
  * **An owned parameter binds `mut` when `Mut` is reachable by peeling an
    arm**, not only when its own type carries it (`type_has_mut_arm`): a
    moved `Ok Mut List<Int> | Err Str` parameter is peeled with
    `o.u1_mut()`, which borrows the binder. The *mode* deliberately still
    reads `type_has_mut`, since an arm's `Mut` is not a claim about the
    parameter — which is what leaves the case below.
  * **The one refused shape** [backend-never-wrong]: a union-typed parameter
    the frame received **borrowed** (`fn f(o: Ok Mut List<Int> | Err Str)`
    renders `&Union2<…>`) whose arm is mutated. There is no `&mut` to give
    and nothing can ask for one — a written `=> o: Mut` is refused by the
    checker, since the `Mut` is the arm's claim and not the parameter's — so
    the emitter **reports**, naming the remedy (take the payload as its own
    `Mut List<T>` parameter and check the arm at the call site). Kotlin
    compiles and mutates the caller's value here, so this is a real
    divergence closed by restriction on one side; lifting it is a deduction
    question, open in ROADMAP.md.
  * **Why it is a rule and not an optimization**: reusing the read form was
    silently wrong. `&mut (p.as_ref().unwrap().clone())` compiles, and the
    mutation lands on the clone — an iterator driven through a narrowed handle
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
* [let-destructure] A **loop pattern** lowers through an `__elemN` temporary:
  the header binds the element, the body opens with one binding per name, read
  off it — `let k = &__elem.0;` / `let who = &__elem.name;`, registered as
  reference bindings [rs-borrow-locals]. By reference because that is the one
  shape that serves an owned element *and* a borrowed one (an iterator hands out
  projections): reads borrow, and an owned use clones exactly as it does for a
  `&T` parameter. A native Rust pattern in the header cannot — `mut k` opts out
  of match ergonomics, so it moves out of a shared reference (`E0507`). A name
  the body **assigns to** takes an owned copy (`let mut a = __elem.0.clone();`),
  since a reference cannot be reassigned and the element is not what the
  assignment means.
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
* [qual-lift] [rs-widen-shadow] A `^` check emits the same test `is` would
  (or `true` when the qualifiers are statically present — qualifiers are
  erased, so widening is a typing act). Where it *peels a wrapper arm*, the
  widened value is bound to a **shadowing local** at the top of the branch
  (`let mut nested = nested.u1().clone();`), so reads of the subject and any
  nested `when` see the inner value. The binding kind is saved and restored
  around the branch, since the shadow is owned where the outer binding may be
  a borrow.
  * **Unless the peeled payload is `Mut`** (2026-09-20): then the shadow is a
    **mutable borrow into the storage** (`let d = d.u1_mut();`, bound
    `BindKind::RefMut`, no `mut` on the binder), because an owned clone made a
    mutation inside the branch vanish when the branch ended — while Kotlin,
    which casts the storage, kept it. [rs-narrow-mut] carries the whole set of
    mutable sites and the one shape still refused.
  * **A lift with a binding needs no shadow** (2026-09-21): `is ^Ok value`
    materializes the lifted value into `value` instead, through the
    `is`-binding path. A lift of a **qualifier only** binds the subject itself
    there, since qualifiers are erased — the payload read the wrapper case uses
    would be wrong for it, and was: it assumed an `Option` in front and emitted
    `list.as_ref().unwrap().clone()` for a plain `&mut Vec`.
  * Without the shadow the emitted code compiles and is **wrong**: the nested
    `match` scrutinizes the outer wrapper, whose arm 0 is the one the outer
    test already took, so the second inner branch becomes dead code. Caught
    by running the feature's own demo (`Display` on the generated union had
    been masking it in the printed output).
  * `^` on a *projection* (`p.result is ^Ok`) is a reported codegen error for
    now: the materialization needs a plain variable to shadow.
* [fn-effects] [rs-fn-effect-params] A fn type's effects are **leading
  `&E` handle parameters** of the closure [rs-handle]: `(s: Str)
  [Logger] -> Str` renders as `&mut impl FnMut(&Logger,
  &String) -> String`, a lambda as `|logger: &Logger, s| …`, and
  the call passes the handle first. Nothing is captured, which is what lets
  the value cross a call that borrows the same effect value.
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
  * The only source today is the release a `for` owes an iterator it owns
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
  effect parameters and any local the body mutates — an exclusivity trap; a
  labelled block captures nothing.
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
    handle's forwarding impl and the generated host skeleton all render
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
  * **An implicit parameter's position follows the same rule** (fixed
    2026-09-21, the ordering round's step 1). A kept non-`Mut` position whose
    type is not a Copy scalar renders `&T`: `?Ordered<T>` is
    `&mut dyn FnMut(&T, &T) -> i32`, the body's `cmp(a, b)` passes `&a, &b`,
    and the adapter that fills the position bridges to the resolved fn's own
    mode (cloning out of the borrow where that fn owns its parameter, which is
    free for a scalar). Until then an implicit's kept position was by value —
    a *move* of what the contract says is kept [fn-contract], so a generic fn
    that compared two values and then used one of them did not compile
    (`E0382`), and the whole comparison capability was unusable over anything
    but a Copy scalar. The same bridging applies to a value written at the
    call site [implicit-override]: a named fn gets the adapter, a lambda binds
    its parameters under the position's convention.
* [rs-cmp-groups] [cmp-groups] The canonical comparison/equality/hashing
  implementations are the host's own operations. `cmp` is
  `Ord::cmp(&a, &b) as i32` — `Ordering` is a fieldless `#[repr(i8)]` enum
  whose discriminants *are* the sign convention Salvo's `cmp` answers, so the
  cast is the whole lowering — written as a path call rather than a method
  call so it works whether the argument arrives owned or borrowed (`&T` has
  its own `Ord`, delegating to `T`'s). `eq` is `==`. A `Str` is compared and
  hashed as `str` (`&s[..]`), which is byte-wise UTF-8 and therefore
  code-point order [kt-ordered] — no runtime helper needed on this side.
  * [obligation-by] A member **stamped** by a `by` clause is an ordinary Rust fn
    whose body is the unrolled Salvo: `pub fn cmp__n(a: &Point, b: &Point) ->
    i32` compares `a.x` with `b.x`, then `a.y` with `b.y`, then answers `0`.
    Calls, adapter closures and `cmp = cmp@Point` values all reach it as a
    named fn, so nothing in this backend learns the member was not written by
    hand — and it takes part in overload mangling like any other body-bearing
    fn. The interim structural identities of a `List` or a tuple
    [col-hashed-ordered] lower to `Ord::cmp`, `==` and a `DefaultHasher` over
    the host value; `mix_hash` to `wrapping_mul(31).wrapping_add(...)`.
  * [cmp-hash-values] `hash` is a block expression holding its own
    `std::hash::DefaultHasher`: `{ let mut __h = …; Hash::hash(&v, &mut __h);
    Hasher::finish(&__h) as i64 }`. One hasher per call, so a `hash` nested
    inside another one is still well-defined, and the value is this host's —
    Kotlin's `hashCode()` answers something else by design.

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
  `intrinsics::handler_member` instead, same shape. A *dependency* is a
  `__dep_E: E` handle field and a trailing `new` parameter — see
  [rs-handle].
* [kt-effect-params]-equivalent: effect dependencies become leading
  parameters `name: &E<...>` [rs-handle]; effect member calls
  dispatch through the parameter (`console.print(...)` — auto-reborrow), and
  callee dependencies thread as arguments (`draw(random_int, console)`,
  with `&mut local` for handlers `use`d in the current scope).
  * [deduce-syntax] **Effect member parameters follow the member's own
    written deduction clause** (2026-09-14): consumed (`=> !s`) is by value,
    kept `Mut` is `&mut T`, kept plain is `&T`, Copy scalars and variadics by
    value. A member has no body to infer from, so [decl-explicit] makes the
    clause mention every non-Copy parameter — the clause *is* the contract.
    One function (`member_param_mode`) answers for the trait method, every
    handler's implementation, the handle's forwarding impl and the argument
    rendering at call sites, because a disagreement between any two of them
    is a rustc type error — and it decides on the *effect's* parameter type,
    so a `T` position stays `&T` in a handler of `Store<Int>`.
    Member fns with their own generic parameters are a codegen error (`dyn`
    traits cannot have generic methods) [backend-never-wrong].
    * Until 2026-09-14 members used the default kept rule regardless (`&T`
      for non-Copy, body clones), recorded as sound-but-unoptimized. It was
      fixed rather than kept for phase 4: a member consuming a **linear**
      token is the shape `Fs.close(s: InStream) => !s` has, and a `&T`
      parameter made the handler clone the token it was meant to consume.
  * [effect-member-overload] **An overloaded member name is suffixed**
    (`close`, `close__2`, …) — Rust cannot overload a trait method at all.
    The name comes from `salvo_core::effect_member_name`, so the trait, every
    handler impl, the handle's forwarding impl, the host skeletons and the
    call sites cannot disagree, and the Kotlin backend picks the same names.
    A call site emits the overload the *checker* resolved
    (`Checked::effect_member_calls`); no recorded resolution where the name
    is overloaded is a codegen error, never a guess.
* [effect-use] `use Handler(...)` emits
  `let <name> = E::shared(Handler::new(args));` (`locked` for a stateful
  handler) and registers `&<name>` in the effect environment for the rest of the scope
  [effect-scope]; ctor arguments are owned (a `use` argument is a move,
  [deduce-infer]). Handler generics are inferred by rustc from the `new`
  arguments (the checker already validated the instance, `use_effects`), or
  spelled as a turbofish where nothing infers them [effect-handler-generics].
  See [rs-handle].
* Handler resolution prefers the checker's effect tables
  (`use_effects`/`effect_calls`/`call_effects`) rendered through
  `rust_ty`, with the same string-keyed environment fallback as the
  Kotlin backend (see COMPLETED.md "Emitter effect-environment fallback"
  under architectural facts).

### Every binding is a handle [rs-handle]

**Built 2026-09-28** (user decisions of the same day, ROADMAP §2b — *one shape*:
handles everywhere, fusion removed, `local` removed — then, the same afternoon,
**keyed on statefulness and named after the effect**). It replaces the fusion
emission of 2026-09-04/14 (`__Fx_N` structs, `__Has_E` accessors, `__Prov_…`
conjunctions, `__Deps_H`/`__Impl_H` for dependent handlers, `__Prov_H` actor
providers, `__Hs_…` handle bundles) and the monitor apparatus of 2026-09-19/20
(`__Mon_E` over `Box<dyn __Share_E>`, `__Lock_E<H>`, `__Arc_H`/`__Shared_H` for
threadsafe hosts). COMPLETED.md shows the shape (under "Current architectural
facts worth knowing") and keeps the history ("One shape for effects").

* **An effect emits two traits and a handle.** The traits are what handlers
  implement, and their names are the mangled ones; the handle **carries the
  effect's name** — it is what a reader of the output sees on every fn
  parameter, `use` local and dependency field:

  ```rust
  pub trait __Stateless_Console: Send + Sync { fn print(&self, message: &String); }
  pub trait __Stateful_Console: Send        { fn print(&mut self, message: &String); }

  pub struct Console { inner: __Inner_Console }
  pub enum __Inner_Console {
      Shared(std::sync::Arc<dyn __Stateless_Console>),
      Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Console>>),
  }
  impl Clone for Console { … }                          // an `Arc` bump either way
  impl Console {
      pub fn shared<__H: __Stateless_Console + 'static>(inner: __H) -> Self { … }
      pub fn locked<__H: __Stateful_Console + 'static>(inner: __H) -> Self { … }
      pub fn share_shared(inner: Arc<dyn __Stateless_Console>) -> Self { … }
      pub fn share_locked(inner: Arc<Mutex<dyn __Stateful_Console>>) -> Self { … }
      pub fn print(&self, message: &String) {
          match &self.inner {
              __Inner_Console::Shared(h) => h.print(message),
              __Inner_Console::Locked(h) => h.lock().unwrap().print(message),
          }
      }
  }
  ```

  The handle's members are **inherent `&self` methods** dispatching on the
  arm, so a fn declaring `[E]` takes `e: &E` — a shared borrow, which also
  means two handles of one instance in one call never conflict and the
  `E0499` hoisting around `clock.to_instant(&clock.to_tick(i))` is gone.
  Generic exactly as the effect is (`Random<T: 'static>`); construction
  sites name a generic instance's type arguments outright
  (`Random::<i64>::locked(…)`). Emitted for **every** effect, actor effects
  included, used or not. Members with their own generics are skipped as the
  traits skip them. The locator face of a mutable-lending member (`m__loc`,
  [rs-loc]) is dispatched too: an effect member lends from a *parameter*,
  never from handler state, so the borrow outlives the guard legitimately.
* **Statefulness decides the arm** — `salvo_core::handler_is_stateful`, the
  one predicate both backends and the deadlock graph read, off the
  declaration: a `state` field; a member that mints a `replyto` (the parked
  table is state the mint writes — `Checked::parking_handlers`); a fn-typed
  constructor parameter (a stored `FnMut`, and a lambda may mutate what it
  captured); a platform handler without `threadsafe`. An `intrinsic` handler
  is trusted stateless. The generated actor fields (`__addr`, `__parked`) do
  not count: the actor body owns the instance and writes them itself, so an
  actor handler with no state fields bound inline is `Shared` like any other.
  A **stateless** handler implements `__Stateless_E` with `&self` members
  (`Emitter::member_receiver_mut`) and is shared as `Arc<dyn …>` with no
  lock; a **stateful** one implements `__Stateful_E` with `&mut self` and sits
  behind `Arc<Mutex<_>>`. Stateless by construction: the send stub `__Stub_E`
  (so a send through a `use addr` binding blocks only on the mailbox, never
  on a lock another sender holds while blocked on that mailbox), the mixed
  handler's façade `__Fac_H` (so a caller waiting on the servant holds
  nothing), intrinsic handlers, `threadsafe` hosts. A generic handler's
  parameters carry `Clone + Send + Sync + 'static`, since the instance sits in
  an `Arc`; every Salvo type satisfies them.
* **A `use` is the handle**: `let e = Console::shared(StdOutConsole::new());`
  / `let random = Random::<i64>::locked(CyclicRandom::new(…));` — the local
  *is* what everything downstream receives. A multi-face handler is one
  `Arc` (of a `Mutex` when stateful) and one handle per face (`let __inst =
  Arc::new(Mutex::new(H::new(…))); let a = A::share_locked(__inst.clone());
  let b = B::share_locked(__inst.clone());`), so a **stateful multi-face
  handler is one lock behind several effect types** — the shape
  [monitor-handler] used to refuse. `init` runs on the instance before it is
  wrapped. A `use addr` of a plain effect binds the value (an `Addr<E>` *is*
  `E`); of an actor effect, `E::shared(__Stub_E::new(addr))`.
* **A fn declaring `[A, B]`** is `fn f(a: &A, b: &B, …)`, one parameter per
  effect **in the order of `Checked::fn_effects`** — inherited effects (from
  fn-typed parameters, [fn-effects]) first, then the written list — and a
  call passes `&a` for a local or a captured field, the parameter itself for
  a parameter. The checker's `call_effects` follows the same order, since the
  emitters pass one argument per entry (the two disagreed until 2026-09-28;
  fusion had hidden it behind a single fused argument).
* **A dependent handler** ([effect-handler-deps]) holds one `__dep_E: E<…>`
  field per declared dependency, whatever the dependency's kind — plain,
  actor effect (a handle over the send stub), or generic instance
  (`__dep_Store: Store<i64>`, or `<T>` for a generic handler: the cut that
  refused a generic dependent handler is gone) — as trailing `new` parameters
  in declaration order. Members reach them as `self.__dep_E.member(…)`
  through the handle's `&self` methods, so a **stateless** dependent handler
  stays stateless. A `use` clones the scope's handles into `new`
  (`Stamped::new(logger.clone(), clock.clone())`) whether the effect arrived
  by a `use` in the same function or through the enclosing *signature*
  ([spawn-inherit]'s lift needs no bundle: the parameter is the handle); a
  `with` item is a private instance behind its own handle
  (`Clock::shared(FixedClock::new())`, `locked` when stateful).
  **Interception** ([effect-intercept]) is the same shape: `let logger2 =
  Logger::shared(Stamped::new(logger.clone()))` — the previous registration's
  handle is captured before the interceptor is constructed, which is "binds
  strictly outward" in emission.
* **A spawn** hands the actor body the same struct: `__Actor_H { handler: H }`
  with the handler's fields the handles, `__dispatch` calling
  `__Stateful_E::member(&mut self.handler, …)` (or `__Stateless_E::…`, by the
  handler's statefulness) for every handler, dependent or not; the clause's
  items and the inherited dependencies are trailing `new` arguments exactly
  as at a `use`. A **monitor spawn** answers `E::locked(H::new(…))` /
  `E::shared(…)` — no scheduler, no mailbox, no pool. A **mixed spawn**
  answers `E::shared(__Fac_H { __addr, … })`: the façade is stateless, so its
  wait on the servant holds no lock.
* **A fn value with effects** is `impl FnMut(&A, …, args)`: a lambda takes
  each as a typed leading parameter, and the adapter for a named fn forwards
  it (`|__fx0: &Logger, mut __a0| shout(__fx0, __a0)`), ignoring what the
  declaration does not need. **A task body** takes its inherited effects as
  *owned* handles (the mint's closure is `move` and `'static`) and clones
  them into nested mints.
* **`threadsafe platform handler`** ([threadsafe-platform]) is the declared
  statefulness of a host the compiler cannot see: a `threadsafe` host's
  skeleton implements `__Stateless_E` with `&self` receivers and the `use`
  binds `E::shared(H::new(args))` — rustc checks the half of the contract it
  can, since a field that is not `Sync` does not compile — and an undeclared
  host implements `__Stateful_E` with `&mut self` behind `E::locked(…)`.
  `std/platform/net.rs` (`HostTcpTransport`) is written in the first form.
* **Fn-typed constructor parameters and state** ([rs-fn-field]): a written
  fn-typed constructor parameter (`handler Derived(step: (n: Int) -> Int)`)
  is stored like an implicit — `Box<dyn FnMut(i32) -> i32 + Send>`, arriving
  as `impl FnMut + Send + 'static`, called `(self.step)(…)` — and makes the
  handler stateful; a struct's fn-typed field is `Arc<dyn Fn… + Send + Sync>`
  (it was `Rc`). A Salvo lambda captures by value, so the bounds hold for
  every value the program can build, and the checker's "holds a function
  value" unsendability arm no longer applies to handler state
  (`unshareable_reason`); it still applies to message payloads and task
  captures (`unsendable_reason`).
* **The handle is `Clone`, not `Copy`** — unlike the `usize` addr — and the
  checker treats every `Addr` as freely reusable, so an owned read of a
  plain-effect addr **clones** (`emit_owned`'s ident arm). Both arms are
  `Send + Sync`, which is the sendability the checker promises at a spawn
  [actor-sendable].
* **Rust's `Mutex` is not reentrant, and that is unobservable**: a handler's
  bindings are fixed at construction and its dependencies bind strictly
  outward/earlier, so no path routes back into its own handle; sibling calls
  inside a handler are direct self calls under the one acquisition. A
  poisoned lock (`unwrap`) surfaces as a panic only after another member
  already panicked, which is the fault boundary's business.
* **`Addr<E>` for a plain effect lowers to `E`** (both type paths — checked
  `Ty` and written AST — branch on the effect's declared kind); an actor
  effect's addr stays `usize`. A generic plain effect's addr carries the
  instantiation (`Addr<Random<Int>>` is `Random<i64>`); an uninstantiated
  mention is refused by arity, a leniency path rather than a rule.

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
* [rs-intrinsic-handler] An `intrinsic handler` is a struct + `new()` +
    trait impl, with member signatures from the *effect* declaration and bodies
    from `intrinsics::handler_member`.
* [rs-cargo] [platform-host-deps] **With crates declared, the program is a
  Cargo package.** `write_host_manifest` writes `Cargo.toml` beside the crate
  root: `[package] name = <entry module's last segment>`, `[[bin]] path =
  <crate root>`, an **empty `[workspace]`** — so a target directory nested
  inside another workspace (this repository's `examples/`) is its own root
  rather than an unlisted member cargo refuses — and `[dependencies]` rendered
  verbatim from the merged `HostDeps` (a table spec becomes a
  `[dependencies.<name>]` section; a `path` is already absolute).
  `program_command` then runs `cargo build --quiet --manifest-path … --target-dir
  <target>/.salvo_bin` — with the cargo `[rust] cargo` names [host-tool] — and
  launches `.salvo_bin/debug/<name>`; `entry_hint`
  names the cargo command. With **no** crates declared nothing changes: bare
  `rustc`, no manifest — and a manifest this backend wrote earlier (recognised
  by its first line, `CARGO_HEADER`) is removed, a hand-written one left alone.
* [rs-platform-host] [platform-tree] [cli-platform] The host file for module
  `M` is `platform/<M>.rs`, mounted from the crate root as
  `#[path = "platform/<M>.rs"] pub mod platform_<M>;` — the module's own mod
  name with a `platform_` prefix, so a host and the module it implements for
  can never collide [rs-crate].
  * Every reference is a fully qualified `crate::…` path rather than an
    import: the host is a mounted module, and `<path>` is `crate` for the
    crate-root module and `crate::<mod_name>` otherwise. A host struct
    belonging to another module's host file is reached as
    `crate::platform_<N>::<E>Host`. `module_mod_names` is shared with
    `emit_program` so the skeleton and the mounting cannot disagree on a
    name.
* [rs-wire] [wire-format] The wire runtime is `runtime/wire.rs`, mounted as
  `crate::wire` when a codec is generated or `encode`/`decode` lowered: a
  `__Wire` trait (`__enc(&self, &mut Vec<u8>)`, `__dec(&mut __Reader) ->
  Option<Self>`) with impls for the scalars, `String`, `()`, `Vec<T>`
  (`List`, arrays **and** `Bytes` — one impl, since `u8` is one byte),
  `Option<T>`, and tuples to arity 8; `salvo_encode`/`salvo_decode` (the
  latter refusing trailing bytes). Unions get `impl<T1: __Wire, …> __Wire for
  UnionN` in the unions file when the wire is mounted. Every struct with a
  wire form gets `impl __Wire for S` beside it (generic ones conditional on
  `T: __Wire`), every actor protocol with one gets `impl __Wire for __Msg_E`
  plus `pub const __PROTO_E: &str`. `encode(v)` lowers to
  `salvo_encode(&v)`, `decode<T>(b)` to `salvo_decode::<T>(&b)` with `T` from
  the call's checked type argument.
* [rs-wire] [addr-routable] The scheduler's wire section: `ActorState` gains
  `node`, `bits`, `remote: Option<RemoteRef>`, `credits`, `granted`, `decode:
  Option<MsgDecoder>`; `Sched` gains `node_id`, `hosted_nodes`, `proxies`,
  `routes` (node → encoded endpoint), `wires` (node → outbound hook),
  `exported_tasks`; `Target::Remote { node, kind, id, bits }` joins the reply
  targets, `Target::Task` holds its body behind `Arc<Mutex<Option<_>>>` (so a
  crossing token can move it into the exported table from a `&self` encoder)
  plus its answer's `ReplyDecoder`, and `Entry::ReplyRaw(slot, bytes)` carries
  a wire reply until the activation decodes it. `impl __Wire for usize` and
  `for SalvoReply` live in `wire.rs` and call the scheduler, so mounting one
  mounts the other. Generated code: `salvo_spawn(pool, bound, body,
  __DECODE_H)`, `salvo_send_wire(addr, msg, __PROTO_E)` for every send on a
  protocol with a wire form (`salvo_send` otherwise), `salvo_reply_wire::<T>
  (tok, v)` for a `send(reply, v)` whose payload has one, `salvo_waiter_decoder
  (wid, dec)` after every `salvo_waiter()`, `salvo_mint_task(pool, body, dec)`;
  the actor body implements `decode_reply` and a free `__DECODE_H` constant
  (free, since a generic dependent body could not name an associated const).
  Frames staged under the lock go out through a thread-local outbox flushed
  after every release (`flush_out`), including in `run_job` for the credit a
  dequeue grants.
* [rs-wire] [effect-generic-decl] [actor-group] The emitter builds against
  `salvo_core::erase_effect_generics(program, &erased_generics(program))`:
  `rust_ty`/`emit_named_type` drop the type arguments of an erased name, and a
  call to an erased fn takes no turbofish (its `generics` are empty in the
  copy, so `effect_instance_turbofish` has nothing to emit). `#![allow(…)]`
  gains `non_upper_case_globals` for the free `__DECODE_*`/`__PROTO_*`
  constants of erased generics. Runtime: `salvo_pending(addr)` (queue depth
  locally, `granted` on a proxy; GRANT decrements it). [node-group] The
  group protocols are std's (2026-10-02, the runtime record): the runtime
  carries them as CONTROL frames (kind 4: to, from, channel, payload) to
  `Sched.controls[(node, channel)]`, registered by `salvo_watch_control`
  with a `ControlOf = fn(u64, Vec<u8>) -> SalvoMsg` builder, and keeps only
  `salvo_send_control`, `salvo_control_frame` (to 0, for a HELLO),
  `salvo_node_left`, `salvo_local_protocols` and the `peer_protocols` store
  (`salvo_set_peer_protocols`, `salvo_peer_protocol`). Intrinsics lowered: `protocol<E>()` — the `Protocol { name,
  hash }` literal from the per-protocol hash constant, for the written type
  argument at a direct call and for the **resolved position's** `E` when it
  fills an implicit `?protocol: () -> Protocol<E>` (`emit_implicit_args`
  reads `ImplicitArg::Resolved.want`; the adapter is `&mut || Protocol {
  … }`, a generic caller forwards `&mut *protocol`) [implicit-intrinsic] —
  `watch_control`, `send_control`,
  `control_frame`, `node_left`, `local_protocols`, `set_peer_protocols`,
  `peer_protocol`, `pending`, `node_of`, and `eq(Addr, Addr)` as `==` on the handle.
  `actor_group<E>` is ordinary Salvo since 2026-09-28 [actor-group] (two
  overloads, `actor_group` and `actor_group__2` in the output).
* [rs-wire] [route-stub] Runtime: the views live in the routing service
  (`runtime.routing`'s `view_set`, `view_members` sorted by `(node, actor)`,
  `view_version`, `view_refresh`, `view_wait`); `salvo_key_hash` (FNV-1a 64
  over the wire bytes) stays host-side. Intrinsics: `view_set`,
  `view_members`, `view_version`, `view_refresh`, `view_wait`, `key_hash`
  (over `salvo_encode`). `use route_any(g, c)` emits the construction of
  the checker's `route_stubs[site]` handler exactly as a written `use
  __Route_E(g, c)` would. An erased effect's types render without arguments
  everywhere an instance is split into base and args (`ty_effect_parts`,
  `handler_dep_effects`, the `use` turbofish) — `RouteSelector`, `dyn __Stateful_RouteSelector`,
  `Sharded::new()`. `while true` lowers to `loop`.
* [rs-actor] [handler-init] `init` is one more private member: `__Priv_H::Init`,
  an inherent `fn init(&mut self)`, dispatched by `__dispatch_priv`. A spawn
  emits `salvo_send(__a, Box::new(__Priv_H::Init))` right after `salvo_spawn`
  and before the addr is answered. A `use` wraps the construction in `{ let
  mut __h = H::new(…); __h.init(); __h }` before the handle takes it — a
  dependent handler's dependencies are its own fields, so `init` is a plain
  method like every other member [rs-handle].
  `self@Face` lowers to `self.__addr.expect(…)`; the `watch_control`
  intrinsic builds a `__Priv_{current handler}::Control` variant, since a
  control frame's payload arrives as the handler's own private member.
* [rs-actor] [actor-private-send] A private send member is an **inherent
  method** on the handler struct (`impl H { fn k(…) }`) with owned
  parameters (its written clause is all-consumed). `__Priv_H` is the
  handler-keyed enum of its private messages; `__Actor_H::handle` tries it
  after the faces' enums, `__dispatch_priv` calls the method, `resume`
  rebuilds a `__Priv_H` for a
  `ContTarget::Private` variant of `__Cont_H`, and `decode_reply` covers it
  by its answer type. `k@self(…)` builds `__Priv_H::K(payload)`, or the
  method call inline when `__addr` is unset.
* [rs-abi] [platform-abi] **The Rust host crate** is `emit_abi`: the ordinary
  emission in ABI mode, with the crate layout the build has, so an
  implementation file's `crate::…` paths mean the same in both. Mod names are
  computed over the build's modules (so they are the build's), the entry
  module's kept declarations form the crate root `lib.sv.rs` as its whole file
  does in the build, every other kept module is mounted under its build name
  from `<module path>.sv.rs`, the runtime modules from `salvo/<name>.sv.rs`,
  and each project module with platform declarations has its implementation
  file mounted as `platform_<m>` from `<module path>.rs` — written before the
  skeleton exists, since a missing file is rustc's to report. A `use
  crate::<m>` naming a module the host crate does not mount is dropped.
  `Cargo.toml` names `lib.sv.rs` as the library, declares an empty
  `[workspace]` (so a root inside another workspace is not claimed by it),
  and lists the manifest's crates. Verified by rustc over a root alone
  (`the_host_project_compiles_on_its_own`) and `cargo check` on aws's root.
* [rs-host-abi] [platform-abi] **What host code may rely on**, in one place
  (the rules it summarizes are the ones cited):

  | Salvo | Rust, as host code sees it |
  |---|---|
  | module `a.b` | `crate::a_b` (items `pub`); its companion is `platform/a/b.rs` [rs-platform-host] |
  | `Int` `Long` `Float` `Double` `Bool` `Char` `Byte` | `i32` `i64` `f32` `f64` `bool` `char` `u8` [type-basic] |
  | `Str`, `Bytes` | `String`, `Vec<u8>` (`Mut` erases) |
  | a literal, a union of one base's literals | its base (`"A" \| "B" \| Other Str` is a `String`) [type-literal] |
  | `List<T>` | `Vec<T>` |
  | `Map<K, V>`, `Set<T>` | `crate::collections::SalvoMap` / `SalvoSet` — insertion-ordered; build one with `.collect()` or `SalvoMap::from_entries::<HostHash, HostEq, _>(entries)` [rs-collections] [platform-check] |
  | `SortedSet<T>`, `SortedMap<K, V>` (canonical ordering) | `SalvoSortedSet` / `SalvoSortedMap`; build one with `.collect()` (under `HostOrd`, which is Salvo's order for every type that may cross) [platform-check] |
  | a union, to build | its factories: `FsError::not_found(…)`, `ReadToStr::ok(…)` [platform-factory] |
  | `T?` | `Option<T>` [rs-option] |
  | a union of *n* ≥ 2 runtime arms | `crate::unions::UnionN<A, …>` with variants `U1`…`Un` in runtime-arm order [union-arm-identity]; `Some(…)` around it when it has a `None` arm |
  | `struct S { f: T }` | `pub struct S { pub f: T }`, built with a literal of every field; a dot-name `A.B` is `AB`; a field that is a Rust keyword is `r#f` |
  | `Checked<T>` | `crate::core_checked::Checked { value: T }` |
  | `Reply<T>` parameter | `crate::scheduler::SalvoReply`; `.hosted()` answers the `SalvoHostReply` whose `send(v)` may run on any thread, exactly once [platform-reply] |
  | `InStream` / `OutStream` | `crate::stream::InStream { handle: i64 }`; the table is Salvo (`runtime.streams`); host code reaches it through `crate::hoststreams::salvo_stream_register_in/out` and `salvo_stream_take_in` (an `Arc<Mutex<SalvoIn>>` reader) [stream-table] |
  | `platform handler H(p: T) of E` | `pub struct H` with `pub fn new(p: T) -> Self`, implementing `crate::<module of E>::EPlatformSync` (`&self`) when `threadsafe`, `EPlatform` (`&mut self`) otherwise [rs-platform-handler] |
  | a member parameter | kept non-`Copy`: `&T`; kept `Mut`: `&mut T`; consumed, or `Copy`: `T` [rs-borrows] |

* [rs-mod-use] [mod-use] A module-level `use` emits `fn __module_use_N() ->
  &'static E` with a `static CELL: OnceLock<E>` whose initializer is what
  the `use` statement would emit; every fn of the module starts its effect
  environment with the accessor call as the entry, so a call threads
  `__module_use_N()` where a declared effect threads its parameter.
* [rs-host-fields] A struct whose fields reach a host value — any platform
  type, or a `Reply` — derives what those support: no `Debug`/`PartialEq`
  (a hand-written `Debug` prints such a field as `<fn>`, as for a fn field),
  and no `Clone` at all when the value is linear (a linear platform type, a
  reply token). `host_field_limits` walks type arguments and named structs'
  fields.
* [rs-linear-move] A binding taking a linear payload out of one **arm of a
  union** held in a local (`while r is Full f`) moves it with a `match`, as
  the plain-optional shape moves with `unwrap` (2026-10-02); so does a
  narrowed linear arm handed on (`linear_move_unwrap`), and destructuring a
  narrowed linear struct (`let {a, b} = x`) — which closes ROADMAP 0c item 7.
* [rs-platform-type] [platform-type] The declaring module re-exports the
  host's struct — `pub use crate::platform_<m>::Name;` — so every mention is
  the ordinary path, beside a static assertion of the kind's contract
  (`const _: fn() = || { fn __contract<T: Send + 'static + Clone>() {} … }`,
  `+ Sync` for `threadsafe`, no `Clone` for `linear`), which makes a host
  type breaking it rustc's error at that line. The skeleton is `pub struct
  Name {}` with `#[derive(Clone)]` for the copyable kinds.
  * [platform-iterable] A `for` over an `iterable platform type` is `for x in
    crate::platform_<m>::each(&xs)` (by reference), `each_mut(&mut xs)` when
    the body writes a field of the element, `into_each(xs)` for a move-mode
    loop over a container, and `each(&xs).map(|__x| __x.clone())` for any
    other by-value loop, including a value-position one. The contract asserts
    each signature at a sample instantiation (`i32` for every parameter).
    The locator variant's indexed loop [rs-loc] stays `List`'s.
* [rs-effects] [fn-contract] An effect member's fn-valued parameter is
  `&mut dyn FnMut(…)`, and a `once` one `Box<dyn FnOnce(…) + '_>` boxed at
  the call (`member_fn_param_ty`): effect traits are used as `dyn`, and an
  `impl` parameter made the trait not object-safe — every effect with a
  fn-valued member was a rustc error until 2026-10-02. A top-level fn keeps
  `impl FnMut`/`impl FnOnce` and monomorphises.
* [rs-platform-never] [platform-never] `Never` is `()` everywhere in Rust
  output except a platform signature's result: a `platform fn`, its wrapper
  and skeleton, and an `EPlatform`/`EPlatformSync` member are `-> !`
  (`return_is_never` in `emit.rs`). The adapter's impl of the effect's own
  trait keeps `-> ()` and forwards the host's `!`, which coerces.
* [rs-platform-factory] [platform-factory] A named union gets `pub type FsError =
  Union7<…>;` (Rust otherwise spells union aliases out) and `impl FsError {
  pub fn not_found(value: NotFound) -> Self { crate::unions::Union7::U1(value)
  } … }` — an inherent impl on one instantiation of a crate-local type. A
  signature's union gets `pub struct ReadToStr;` with an `impl`. Two named
  unions over the same instantiation with a shared arm name collide in rustc
  rather than silently.
* [rs-platform-check] [platform-check] **Collections** (D10 C1, C2): Rust's
  types promise insertion order and the ordering, so a plan's `Shape` nodes
  are dropped (`BoundaryCheck::without_shapes`); `FromIterator` on
  `SalvoSet`, `SalvoMap`, `SalvoSortedSet` and `SalvoSortedMap` builds each
  under the canonical identity, the only one a host may return.
* [rs-platform-check] [platform-check] A check renders as statements over a
  reference (`let __c = &__r;`), panicking with `"salvo: … [platform-check]"`
  and the value's `{:?}`: closed literals as `matches!(v.as_str(), "a" | "b")`
  (or `*v` for numbers and `Bool`), a state qualifier as a call of its
  `qualifies` by crate path (`*v` for a Copy subject), elements through
  `.iter()`, a nullable value through `if let Some(..)`, a union arm through
  `if let crate::unions::UnionN::Uk(a) = v`. A reply is passed as
  `reply.checked(Arc::new(|any: &dyn Any| …))`, downcasting to the payload's
  Rust type; `SalvoReply`'s `check` runs in `send` and in
  `SalvoHostReply::send`.
* [rs-platform-handler] [platform-handler] Beside an effect `E` some reachable
  platform handler implements, `E`'s module emits the host-facing traits —
  `EPlatform: Send` (`&mut self`) when a serialized handler needs it,
  `EPlatformSync: Send + Sync` (`&self`) when a `threadsafe` one does — and
  the adapter `pub struct __Platform_E<T>(pub T)`, with `impl<T: EPlatform>
  __Stateful_E` and `impl<T: EPlatformSync> __Stateless_E` forwarding each
  member [platform-abi]. A `platform handler H of E` in module `M` emits
  `pub type __Platform_H = <E path>::__Platform_E<crate::platform_<M>::H>`
  with an inherent `new(p: T)` building the host struct (an inherent impl on
  one instantiation of a local type, so several handlers of one effect each
  have their own), and the `use` site constructs `<M path>::__Platform_H::
  new(args)`, wrapped in the effect's handle like any construction
  [rs-handle]. `M` is the module that *declared* the handler
  (`Symbols::handler_modules`). Emitted only when the host file exists (or
  in a host project).
  * The skeleton is `pub struct H { p: T, … }` with `impl H { pub fn new(p:
    T, …) -> Self }` and an impl with every member stubbed — of
    `<path>::EPlatform` (`&mut self`) for an undeclared handler, of
    `EPlatformSync` (`&self`) for a `threadsafe` one [threadsafe-platform] — named after
    the *handler*, and with a `new` because the `use` site calls one,
    exactly as it does for a generated handler struct.
  * A `use` whose declaring module has no host companion is a codegen error
    naming `salvo platform generate` [backend-never-wrong]; std's companion
    is shipped in `std/platform/` rather than generated [platform-tree].
  * The skeleton opens with the **`use` lines its own signatures need**: the
    declaring module's items, `crate::unions::*` when a member's result is a
    union, the ordered collections when one appears, and the module of the
    effect being implemented when that is elsewhere. A host file is a module
    of the same crate, so without them the skeleton does not compile — which
    stayed invisible until a `platform handler` whose members trade in more
    than primitives arrived (`HostRawFs`, 2026-09-14).
  * **Sharing follows the declared contract** [threadsafe-platform]
    [rs-handle]: a `threadsafe` host implements `EPlatformSync` with `&self`
    receivers and its `use` binds `E::shared(H::new(args))` — no lock, and
    rustc refuses a host whose fields are not `Sync`; an undeclared host
    implements `EPlatform` with `&mut self` behind `E::locked(…)`, so a
    host that did not claim safety behaves identically on both backends and
    pays only the lock.
  * **The skeleton prints the contract** in both shapes: the threadsafe one
    opens with the signing comment and implements `<effect_path>::
    EPlatformSync` with `&self` receivers; the undeclared one says the
    compiler serializes the instance and implements `EPlatform` with
    `&mut self`. Regenerating after toggling the word changes the receivers.
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
* [rs-runtime-host] **The scheduler itself is Salvo** (2026-10-03,
  [runtime-sched]): std's `runtime` module and its services, emitted like any
  module. What Rust writes by hand is in std's platform root:
  `std/platform/runtime.rs` (`HostRuntime`, `Parker` over
  `std::thread::park`, daemon threads, the fault boundary over
  `catch_unwind`, `Dyn` as `Box<dyn Any + Send>`, `Body`, `Slot`, the `here`
  thread-local) and `std/platform/runtime/{routing,streams}.rs` (frame
  codecs, `HostIn` over `Box<dyn Read + Send>`). `runtime/scheduler.rs` is
  the entry points generated code calls (`salvo_spawn`, `salvo_send`,
  `SalvoReply`, …) as calls into the core, plus `scheduler::registry()`, the
  proxies the wire codecs read; about 590 lines, from 2,720 before the port.
  `runtime/hoststreams.rs` is host code's entry points into the stream table.
* [rs-deque] [col-deque] `Deque<T>` and `Mut Deque<T>` are
  `std::collections::VecDeque<T>`; the surface is `push_back`/`push_front`,
  `pop_front`/`pop_back`, `remove`, `get`/`front`/`back`, `len() as i32`.
  An empty `deque_of()` spells its element type from the call's resolved type
  argument (`VecDeque::<T>::new()`), since rustc cannot infer it from an
  unconstrained temporary.
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
  * [linear-container] The obligation surface added two methods, both about
    *not dropping*: `replace(key, value) -> Option<V>` (`insert` answers
    nothing, so it cannot be the write for a map of obligations) and
    `into_values() -> Vec<V>`, which is what `drain(map, each)` walks.
* [rs-stored-implicit] [cmp-carry] **A capability the callee *keeps* arrives
  owned**, as `std::sync::Arc<dyn Fn(…) -> … + Send + Sync>` rather than the
  usual `&mut dyn FnMut` — the same convention exception an iterator fn's
  callbacks get [rs-iter-pass], and for the same reason: what receives it
  outlives the call. What keeps one is a **keyed container built inside a
  generic function** (landed 2026-09-26): there the identity is a capability the
  function was handed, so no marker type can name it and the container has to
  hold the functions themselves.
  * The runtime grew a **value-keyed store per family** beside the
    marker-keyed ones — `FnStore` (hash/eq), `FnSortedStore` and
    `FnSortedMapStore` (cmp) — reached through `SalvoSet::with_fns`,
    `SalvoMap::with_fns`, `SalvoSortedSet::with_cmp` and
    `SalvoSortedMap::with_cmp`. They cannot reuse `HashedStore`/`BTreeStore`
    because `HashBy`/`OrdBy` dispatch their `Hash`/`Eq`/`Ord` impls through
    *types*, and a closure held elsewhere is not reachable from them. The hash
    store's shape is the Kotlin runtime's: a slab in first-insertion order plus
    a bucket index from a Salvo hash to the slots holding it, confirming a hit
    with `eq`. The sorted pair is a sorted `Vec` with `binary_search_by`.
    `Arc` rather than `Box` so a container's clone shares its identity.
  * **The demand is closed under forwarding.** The seeds are the construction
    sites (an expression whose type is a keyed container carrying a
    `FnId::Binder`); a fn that passes its own implicit into a position already
    owned holds an `Arc` for the same reason, so it joins the set
    (`stored_implicit_demand`, mirroring `lend_mut_demand`). At a call, a kept
    position takes `Arc::new(move |…| …)` for a resolved or written identity and
    `Arc::clone(&name)` for a forwarded one.
  * **Its key type carries the store's bounds**: a generic in such a fn is
    `Clone + Send + 'static` rather than the usual `Clone`, because the store is
    a `Box<dyn … + Send>` over owned data. Every Salvo type satisfies both, so
    this only spells at the one site what the container needs.
  * The **intrinsic** identity is not special here: a host `hash`/`eq`/`cmp`
    filling a kept position renders as its own lowering inside the closure, the
    way any intrinsic implicit value does [implicit-intrinsic] — so a generic
    body reaches the host's identity through exactly the same code as a
    declared one.
  * A **mixed** identity — one slot forwarded, another named — is honoured too
    (user decision 2026-09-26): a marker type cannot be mixed into a value pair,
    so the named slot becomes its own closure beside the forwarded handle
    (`Arc::new(move |__a: &T, __b: &T| all_same(__a, __b))`, and
    `<HostEq as SalvoEq<T>>::eq` for the host's). An identity that needs
    implicits *of its own* has nothing to fill them from there and is refused by
    name [backend-never-wrong].
  * [rs-borrows] **A marker calls its identity in the identity's modes.** The
    trait fixes the marker's own parameters as references, while a declared
    identity over a Copy scalar takes its arguments by value — so the marker
    derefs (`all_same(*__a, *__b)`). Splicing the reference through was rustc's
    E0308, reachable the moment a *written* fill became a carried identity
    (2026-09-26): before that, a scalar subject always took the `HostHash`/
    `HostEq` path and no marker was generated for one.
* [rs-mailbox] [actor-mailbox] **The mailbox bound is a generated field on the
  handler**, `__mailbox_capacity: i32`, initialised by `new` from the slot's
  expression — which is exactly where a state field's initialiser is computed,
  and the reason the slot's expressions are confined to constructor parameters:
  in `new`'s scope they are simply in scope.
  * A spawn therefore reads the bound **off the instance**, before it moves
    into the actor body: `({ let __h = H::new(args); let __cap =
    __h.__mailbox_capacity; salvo_spawn(pool, __cap as usize,
    Box::new(__Actor_H::new(__h))) })`. The ordering is the point — the
    alternative (emitting the slot's expression at the spawn site) would need
    the constructor arguments in scope there, and would evaluate them twice.
* [rs-is-hoist] [is-bind-once] **A non-place `is` subject becomes a
  temporary**, read by both the test and the binding: `let mut __is1 = <subject>;`
  before an `if`, and inside a `loop` for a `while` — which is why a `while`
  whose condition binds over a call lowers as
  `loop { let mut __is1 = …; if !(__is1.is_some()) { break; } let mut x = __is1.unwrap(); … }`
  rather than as a `while` with the subject in its condition. `place_storage`
  answers the temporary for that subject's span, so the test, the binding and
  any nested read all agree.
* [rs-state-take] [linear-state] **Taking a container out of handler state is
  `std::mem::take`.** A field behind `&mut self` cannot be moved out (E0507),
  and cloning it would duplicate every obligation inside — so the read the
  checker recorded as a state-field move (`Checked::state_takes`) renders as
  `std::mem::take(&mut self.waiting)`. That is also the honest semantics: the
  field is empty until the member puts something back, which [linear-state]
  requires it to do before returning. `mem::take` needs `Default`, which
  `Vec` and `SalvoMap` have — and a *bare* obligation in state (whose type
  need not) is refused by the checker, so the emitter never meets one.
* [rs-linear-move] [linear-container] **A narrowed linear value is moved, not
  cloned.** Rust's ordinary narrowed read is `x.as_ref().unwrap().clone()`,
  which for `remove_first(waiting)`'s `Option<SalvoReply>` would duplicate a
  one-shot token — and `SalvoReply` is deliberately not `Clone`, so it would
  not even compile. Where the checker recorded a move of a linear value
  (`Checked::linear_moves`), the plain-optional shape renders as
  `first.unwrap()`, moving the payload out; the checker has consumed the
  variable, so nothing reads it again. A narrowed *union arm* keeps the
  existing accessor path, where a linear payload is a `Clone` handle today
  (phase 4's tokens) and a non-`Clone` one is a loud rustc error rather than
  wrong code.
  * **The `is`-binding site too**, since 2026-09-18: `remove_at(pending, i) is
    Reply<Fired> token` binds by moving out of the `Option`. The checker
    records the *binding's* span in `linear_moves` (its use-site path never
    sees a binding), and the emitter takes the moving branch before the
    ordinary narrowed read. Found by [time-manual]'s deadline queue, which was
    a raw E0507 before it — and the same fix removed a silent obligation copy
    from `examples/linearity`'s generated code.
  * The two intrinsic terminals are `into_iter().for_each(f)` /
    `into_values().into_iter().for_each(f)` rather than a `for` loop, for one
    boring reason worth recording: an immediately-applied closure literal
    (`(|r| …)(x)`) leaves rustc with nothing to infer the parameter type from
    (E0282), while a `for_each` argument is typed by the `FnMut` bound.
* [rs-time] [time-types] **The clock readings are a runtime module of their
  own**, `runtime/hosttime.rs` [rs-runtime-source], holding
  `salvo_mono_nanos()` and `salvo_epoch_nanos()` — the whole of the host's
  contribution to std's time surface, since `Duration`, `Instant` and `Tick`
  are ordinary structs over one `i64` each and everything else about them is
  Salvo.
  * **The monotonic origin is process-wide**, a `OnceLock<Instant>` initialised
    by the first reading. It has to be somewhere: Rust's `Instant` is opaque
    and cannot be turned into a number, and an origin *per call site* would put
    two timelines in one program and make `between` answer nonsense.
  * `salvo_epoch_nanos()` answers a **negative** number before 1970 rather than
    saturating, matching the Kotlin side number for number.
  * **The file is named `hosttime.rs`, not `time.rs`** — deliberately, and the
    reason is a trap worth knowing: a runtime file and an emitted *std module*
    share one output namespace, so a module named `time` and a runtime file
    named `time.rs` write the same path, and the second silently clobbered the
    first (a duplicate `pub mod time;` and a pile of missing-symbol errors).
    The general hole is recorded in ROADMAP.
  * **It travels with the host stream table**: `needs_time` is implied by
    `hoststreams.rs`, which ships wherever the scheduler or the wire does.
    `scheduler.rs` is a shim onto the runtime module's core (2026-10-03) and
    holds the `Addr`/`Reply` codecs, so `wire.rs` stands alone; the runtime
    tests compile it inside the files example's generated tree.
* [rs-time] [time-timer] **Deadlines are the runtime module's**, in Salvo
  (2026-10-02): the scheduler has no timer thread, and `HostRuntime.mono_nanos`
  reads `crate::hosttime::salvo_mono_nanos()`, the timeline `tick()` reports.
* [rs-default-path] **A struct literal's type is spelled so it resolves where
  it is written** (2026-10-04): a field default is inlined at the outer
  literal, whose file need not import the type the default names, so a struct
  that file cannot name unqualified is written through its module's path.
* [rs-fn-value-nested] **`f(f(x))` evaluates the arguments first**
  (2026-10-04): a fn value is called through `&mut`, so an argument calling
  the same value borrows it twice (E0499); the arguments go into locals first.
* **Generated code spells `std::boxed::Box`** (2026-10-04): a program's effect
  named `Box` made the generated `Box::new` resolve to its handle type.
* [rs-mailbox] A handler's `__mailbox_capacity` field is **`pub`** (since
  2026-09-18): the spawn site need not be in the same module, and std's own
  `DefaultTimer` is spawned from user code — a private field made that a raw
  rustc E0616 [backend-never-wrong].
* [rs-mixed] **The mixed lowering** [mixed-handler] (SH-1, built
  2026-09-19). A mixed handler splits into two generated types plus the
  servant's runtime parts:

  * **The handler struct is the servant alone**: state + ctor params + the
    actor fields (`__mailbox_capacity`, `__addr`, and `__parked` when a
    send member could be a continuation target); `send fn` members are **inherent
    methods** (no trait declares them), their parameter modes from their own
    written all-consumed clause, so payloads are owned exactly as the message
    enum carries them. Sync members are not emitted here at all.
  * **`__Msg_H` + `__Cont_H` + `__Actor_H`**: the handler-keyed twins of the
    face-keyed message enum, continuation enum and actor body — one variant
    per `send fn` member (continuation variants only for members with
    parameters), `handle` downcasting `__Msg_H` and calling the inherent
    method, `resume` removing the parked continuation and calling the member
    with the downcast answer as its trailing argument [defer-deduction].
  * **`__Fac_H`**: `#[derive(Clone)]`, `__addr: usize` plus the ctor params
    (owned), implementing each plain face with the sync member bodies —
    emitted with the ordinary handler-member machinery (ctor params resolve
    as self fields; state never resolves, the checker confined it). A façade
    send lowers to `salvo_send(self.__addr, Box::new(__Msg_H::Variant(args)))`.
  * **A servant send** [actor-self-send] — a bare sibling call or `k@self(…)`
    in a send member — lowers to
    `salvo_send(self.__addr.expect(…), Box::new(__Msg_H::Variant(args)))`,
    unconditional: a mixed handler is spawn-only, so `__addr` is always
    written. A façade `k@self(…)` lowers exactly as the bare façade send
    does.
  * **The mixed spawn** evaluates ctor args once (`let __cN = …`), clones
    them into the handler, moves them into the façade, reads the mailbox
    bound off the instance, spawns `__Actor_H`, and answers
    `E::shared(__Fac_H { __addr: __a, … })` — the façade behind the effect's
    handle, stateless so lock-free [rs-handle].

* [rs-actor] **Asynchronous effect handlers** lower to three generated
  pieces plus one shipped runtime module, `runtime/scheduler.rs`
  [rs-runtime-source] — emitted, and mounted as `mod scheduler;`, only into a
  program that spawns:
  * **The protocol's message enum**, `__Msg_E`, beside the effect it belongs
    to: one variant per `send fn`, owning its payload. It is the *effect's*,
    not a handler's, because a sender holds an `Addr` and knows only the effect
    it serves — the same reason an actor and a locally `use`d handler are
    interchangeable [actor-types].
  * **The actor body**, `__Proc_H`, beside the handler: a struct owning the
    handler instance (an actor's state *is* the handler's) whose
    `SalvoProcess::handle` downcasts the message enum and calls the member the
    variant names.
    * Member invocation lives in **one** place, a private
      `__dispatch(&mut self, msg: __Msg_E)`: `handle` downcasts into it and
      `resume` rebuilds a call for it.
  * **The parked-continuation table, and the address, live on the handler.** A
    handler of an `actor effect` carries two generated fields, whichever way it
    is bound — a handler is compiled once:
    * `__addr: Option<usize>` — written by `handle`/`resume` from the
      activation's `SalvoCtx` before the member runs, and `None` when the
      instance was bound with `use` instead. That absence is the **self-send's
      discriminator** [actor-self-send].
    * `__parked: HashMap<u64, __Cont_H>` — slot → continuation, emitted when
      the protocol has any member that could be a target.

    They sit on the *handler* rather than on `__Proc_H` because the **mint**
    happens in a member body, which holds `&mut self` on the handler and cannot
    see the actor struct; `resume` reaches them through `self.handler` (user
    decision 2026-09-15, D5-b). The alternative — a table in the runtime —
    would have changed `SalvoProcess::resume`'s decided signature, and the two
    runtimes are the most exactly-mirrored code in the phase.
  * **[effect-handler-multi] A handler of several effects is one actor with one
    dispatcher per protocol.** One `impl E for H` per face (a member that
    implements a same-named member of two faces appears in both impls — Rust
    cannot share a method between two traits, and the signatures are identical
    wherever that is legal, so the body is emitted twice rather than
    forwarded); one `__dispatch_<Effect>` per face, where a single-face handler
    keeps the bare `__dispatch` it always emitted; and a `handle` that asks each
    protocol in turn — `msg.downcast::<__Msg_E>()` hands the box back on a miss,
    which is what makes the chain possible. `spawn` answers `(__a, __a)`: one
    scheduler index, one mailbox, one tuple element per face, so least authority
    costs nothing at run time.
    * A multi-face `use` is one `Arc<Mutex<H>>` and one handle per face
      [rs-handle], so a fn declaring `[A, B]` bound to one handler's two
      faces borrows two handles of one instance — the E0499 the fusion used
      to exist for does not arise.
  * **The continuation enum**, `__Cont_H`, emitted beside the **handler** whose
    members it names — the handler, not the effect, because a mint is lexical
    ([effect-handler-multi]: with several faces a handler's members come from
    several protocols, and `replyto` targets the *handler's*). Its variants are
    named after the effect member each one resumes: one variant per send member
    with at least one parameter, carrying that member's parameters **minus the
    trailing one**. The last parameter is the answer itself [actor-replyto],
    which arrives with the reply rather than being stored — so the variant tells
    `resume` both *which* member to call and *what type* to downcast the answer
    to. A parameterless member gets no variant: there is no answer for a token
    to carry.
  * **`resume`** writes `__addr`, pops the slot (a reply whose continuation is
    gone returns silently), matches the variant, downcasts `value` to the
    trailing parameter's type, and hands a rebuilt
    `__Msg_E::K(captures…, answer)` to `__dispatch`.
  * **A dependent handler's child holds its handles** [rs-handle]: the
    dependency handles are the handler's own fields, so the actor body is the
    same struct for every handler (`__Actor_H { handler: H }`), and the spawn
    passes the clause's instances and the inherited handles as trailing `new`
    arguments — a construction behind its handle, a plain-effect addr as
    itself, an actor addr over its send stub — so a dependency can be a local
    handler in one program and an actor in the next with no change to the
    child.
  * **The three types erase to scheduler handles**: `Addr<E>` and `Pool` are
    `usize` indices, `Reply<T>` is `crate::scheduler::SalvoReply`. Their Salvo
    type arguments have no rendering — the effect an addr serves and the payload
    a token carries are the checker's business, and the message enum is what
    carries payload types into an untyped (`Box<dyn Any + Send>`) runtime.
  * **The shipped runtime speaks the same word**: `salvo_send(addr, …)`,
    `SalvoCtx::addr`, and every internal index is an `addr`. It is emitted
    **into the user's program**, so it shows up in their stack traces beside
    their own code — which is exactly where a second name for one thing would
    cost, and where a real OS pid (a platform handler wrapping process
    management) could sit next to it. "Process" stays the noun for the thing an
    addr names, so `SalvoProcess` and `procs` are untouched.
  * **The forms**: `spawn H(args) on P` →
    `{ let __h = H::new(args); let __cap = __h.__mailbox_capacity;
    salvo_spawn(P, __cap as usize, Box::new(__Actor_H::new(__h))) }`, whose
    value is the addr — the bound is read off the instance because it is the
    *handler's* [actor-mailbox], and a dependent handler's handles are
    trailing arguments of its own `new` [rs-handle];
    `addr.member(args)` →
    `salvo_send(addr, Box::new(__Msg_E::Member(args)))`; `waitfor out: Reply<T>
    { … }` → a block expression that mints a waiter, runs the block, then
    `salvo_wait` and downcasts to `T`; `send(r, v)` → `r.send(Box::new(v))`;
    `pool(n)` → `salvo_pool(n as usize)`; `thread()` → `salvo_thread()`;
    `watch(a, out)` →
    `salvo_watch(a, out, |__reason| Box::new(Exit { reason: __reason }))`;
    `on_idle(p, i)` → `salvo_on_idle(p, i, |__gates, __tokens| Box::new(Idle {
    parked_gates: __gates, parked_tokens: __tokens }))`.
  * **[main-pool] An omitted `on` clause is `salvo_current_pool()`** — a
    thread-local read, so the placement a spawn inherits costs nothing and
    needs no signature. `main`'s thread answers pool 0, the pool it is the
    single worker of; a worker thread answers its own; and an activation
    answers the pool its actor runs on, which is what makes "run where the
    work that created you runs" true inside a member too.
  * **[waitfor-pump] `salvo_wait` serves while it waits.** The runtime's wait
    is a loop, not a condvar park: it runs deliverable work for *this thread's*
    pool — everything except activations of the actor doing the waiting — and
    only parks when there is none. That is what runs main-pool work at all
    (nothing else serves pool 0), and the pumped activation runs nested on the
    waiter's stack. `salvo_thread()` is `salvo_pool(1)`: `Dedicated` is erased
    like every qualifier [qual-erasure], and what it buys is static.
  * **[rs-task] A task mint is a scheduled closure, and nothing else.**
    `replyto k(caps) on P` where `k` is a free `send fn` [free-send-fn] →
    `{ let __c0 = …; salvo_mint_task(P, Box::new(move |__v| k(__c0, …,
    *__v.downcast::<Payload>().expect(…)))) }`, with `salvo_current_pool()` for
    an omitted `on` [task-pool-inherit]. No continuation enum, no slot, no
    `__parked` entry: the closure *is* the continuation, which is why a task
    needs no dispatcher. The captures are bound to `let`s **outside** the
    closure so they are the values as they were at the mint, and the closure is
    `move` so it owns them.
    * The `Box<dyn FnOnce(SalvoMsg) + Send>` this produces is *not* the
      [rs-fn-field] problem: that is a shared, many-shot `Rc<dyn Fn…>` field,
      while this is one-shot, moved once, and `Send`-checked — which is the
      representational fact that made lambdas-as-targets separable from
      [fate-lambda] and let this ship without it.
    * A free `send fn` itself needs no special emission: it is an ordinary
      `pub fn` whose parameters are all moved and which returns `()`.
  * **[pool-fault-sink] `pool(n, sink)`** → `salvo_pool_with_sink(n as usize,
    Some((sink as usize, |__reason| Box::new(__Msg_Faults::Faulted(Fault {
    reason: __reason })))))`. The builder is the same trick as `watch`'s: the
    runtime holds a `String` and cannot construct a Salvo value, so the *pool
    creation site* hands over the constructor. Dispatched on **arity**, since
    both `pool` overloads take an `Int` first and the intrinsic table's key is
    the receiver type.
  * **A `watch` carries its own `Exit` constructor** [actor-watch]. The runtime
    holds a reason `String` and cannot build a Salvo struct, so the watch site
    passes a `fn(String) -> SalvoMsg` alongside the token and the scheduler
    calls it at death — which keeps a watcher's payload byte-identical to an
    ordinary `r.send(Exit{…})` instead of teaching `resume` a special case
    (that special case would have been *silently wrong* the moment a program
    fulfilled a `Reply<Exit>` itself). `Exit` is named **unqualified**, which
    is safe rather than lucky: the file glob-imports every module whose names
    it uses, and a `Reply<Exit>` cannot be obtained in a file where `Exit`
    means something else, so a shadowing declaration and this emission never
    meet.
  * **[actor-on-idle] A quiescence hook carries its own `Idle` constructor**,
    on exactly that precedent: `salvo_on_idle(pool, notify, fn(i32, i32) ->
    SalvoMsg)`, and the counts cross the seam as numbers. What the runtime adds
    for it is an accounting of *undischarged tokens*: `ActorState.owed` counts
    the tokens aimed at an actor, `PoolState.owed` those aimed at a task or held
    by a frame parked on that pool, and `SalvoReply.tracked` is what stops a
    token being counted twice — a delivery clears it, and so does handing the
    token to the scheduler (`salvo_watch`, `salvo_on_idle`), which is why a
    program idling with registrations outstanding reports zero. `fire_idle` runs
    where the scheduler runs dry: in `salvo_wait` *before* the deadlock report
    (firing a hook is progress, so the report is what firing nothing leaves) and
    in `worker` before it parks, which is what fires a hook registered by an
    actor while nobody is waiting.
  * **[waitfor-pump] The deadlock report reads a different predicate from the
    hook** (defect fixed 2026-09-18): `idle()` (`active == 0 && quiet()`) is the
    hook's; `stuck()` (`active == parked_frames && main_waits > 0 && quiet()`)
    is the report's. `parked_frames` counts frames sitting in `salvo_wait` —
    `Here::frame` is what tells an activation or task frame apart from `main`'s
    own thread — `main_waits` counts `main`'s own waits, and `quiet()` adds "no
    waiter is parked on a slot that already holds its value", which is the
    delivery-before-pickup window. `report_deadlock` names the actors parked in
    a wait (`WaiterState::parked`) beside the gated ones.
  * **`replyto k(caps)`** → a block that mints, parks and answers the token:
    `{ let (__r, __s) = salvo_mint(self.__addr.expect(…)); self.__parked.insert(__s, __Cont_H::K(caps)); __r }`.
    `replyto!` differs only in calling `salvo_mint_gated` — the gate is the
    runtime's business, not the emitter's. The `expect` cannot fire: a parking
    handler may only be spawned ([actor-replyto], checked), so its members run
    as activations and `__addr` was written before the body did.
  * **`k@self(args)`** → `match self.__addr { Some(__a) => salvo_send(__a,
    Box::new(__Msg_E::K(args))), None => <inline> }`, where the inline reading
    is `E::k(self, args)`. One field, both readings [actor-self-send].
  * **The forwarding stub**, `__Stub_E`, beside the effect: a struct holding an
    addr that `impl`s the effect trait by sending. `use addr` builds one and
    binds it behind the effect's handle exactly as a handler instance is
    bound [rs-handle]. That indifference is the point: a handler is compiled
    once and bound many ways [actor-use-addr]. A spawn clause's addr becomes
    the same stub, behind a handle in the child's fields.
  * **Still refused** (each a diagnostic, none silent): spawning a **generic**
    handler and a **generic effect** as a protocol.

  * `send(reply, None)` boxes a **typed** `None`: a bare `None` is an
    `Option<_>` rustc cannot infer, so the box is `Box::<Option<T>>::new(None)`
    with `T` read off the token's checked payload (`net`'s
    `Reply<Addr<Inbound>?>`, 2026-09-26).
## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* multi-spread struct literals;
* early `return` inside expression-position lambdas;
* struct literal without an inferable type;
* referencing the `Any` type in emitted positions;
* effect member fns with their own generic parameters;
* a `use` whose effect instance is still generic (an unresolved
  instantiation has no handle type to name, [rs-handle]);
* struct destructuring in `for` patterns (same as Kotlin).

Known acceptable divergences (documented, not errors): extra `.clone()`s
where Kotlin shares references; `Debug`/`Display` formatting of `Option`
values differs from Kotlin's `null` printing (the checker's narrowing rules
make user programs format only unwrapped values).
