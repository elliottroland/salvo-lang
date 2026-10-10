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

* [rs-ir] **The Rust emitter reads the IR** (IR record), and only the IR
  (`crates/salvo-backend-rust/src/ir_emit/`: `mod.rs` for the program and
  names, `decls.rs` for declarations, `body.rs` for statements and
  expressions, `actors.rs` for the actor machinery, `skeleton.rs` for host
  skeletons). Its ownership decisions come from what the IR records: each
  parameter's and argument's **pass mode** (moved, lent, lent mutably), the
  **consume marks** on reads, **`proj` types**, a fn's **`holds`/`borrows`**
  facts, and **alias groups**. The AST is consulted only for intrinsic
  lowering, keyed by the declaration the checker resolved
  ([intrinsic-fn]). What the emitter itself decides is representation and
  Rust idiom: the `UnionN` enums and `Option`, references for `proj` values
  and lent parameters, a `.clone()` where a non-consuming read needs a value
  of its own, the effect handle, and the spelling below.
  * **Every path is fully qualified**: `crate::<mounted module>::Name`, or
    `crate::Name` for the crate-root module, at declarations' mentions, calls
    and constructions alike [rs-imports].
  * **Copy scalars** (`Int`, `Long`, `Float`, `Double`, `Bool`, `Char`,
    `Byte`, and an actor `Addr`) are passed by value; an owned parameter is
    bound `mut x: T`.
  * **Locals are annotated**: `let mut x: T = …;` (a borrow-mode local may
    leave the type to inference, [rs-borrow-locals]). Generated temporaries
    are `__<role>_<N>`, numbered per fn: `__pass_3`/`__step_4`/`__emitted_5`
    (a `for` over a pass), `__nn_3`/`__some_4` (`!`), `__safe_N`, `__elv_N`,
    `__old_N`, `__spread_N`, `__subject_N`, `__narrowed_N`, `__loop_N`,
    `__elem_N`, `__tmpN`, `__argN`/`__partN`, `__hN`/`__lN`/`__cN`/`__pmN`
    (element handles), `__use_N`/`__handle_N` (a `use`).
  * **Literals carry their type**: `1i32`, `5i64`, `2.5f64`, `0.5f32`;
    a string literal is `String::from("…")`.

## Assertions

* [assert-trap] [rs-assert-trap] A failed assertion is a **panic** whose message
  is Salvo's. `assert!(c, m)` is `if !((c)) { panic!("salvo: {} at <module>:<line>:<col>", m) };`
  and `unreachable!(m)` is the `panic!` alone (a literal message is spliced into
  the format string: `panic!("salvo: never at main:23:16")`), which types as `!`
  and so stands wherever a value is expected. The message composition sits
  *inside* the panic, so a written message is built only when the assertion
  fails.
  * `expr!` is a block that tests, panics and answers the payload:

    ```rust
    {
        let mut __nn_3: Option<&crate::Fighter> = crate::named(&roster, &__tmp1);
        if __nn_3.is_none() {
            panic!("salvo: value is absent at main:192:15");
        } else {
            let mut __some_4 = __nn_3.unwrap();
            __some_4
        }
    }
    ```

    The block is a `let`'s value or an argument as it stands. A handle minted
    by a locator ([rs-elem-mut]) traps with `.expect("salvo: value is absent at …")`
    on the locator instead — same text, same timing.
  * The location is the **module path**, not the file name — a file's display
    name depends on the loader, and emitted output must not.
* [rs-opt-borrow] `expr!` on an owned `Option` held by a **local** reads it
  *through a borrow*: the block binds `let mut __nn = &x;` and
  `let mut __some = __nn.as_ref().unwrap();`, and answers `__some` where the
  position borrows, `__some.clone()` only where it owns. Where the position
  mutates it binds `let mut __nn = &mut x;` and
  `let mut __some = __nn.as_mut().unwrap();`, and answers `&mut *__some`. A
  Copy payload unwraps directly (`__nn.unwrap()`, a value in every position),
  and so does an `Option<&T>` operand, whose payload already *is* the borrow.
  Two `!`s on one local are therefore two reads, as the checker treats them —
  reading an optional is free.
* [rs-read-mode] Whether a read answers a borrow or a value is the
  **position's** question: a lent position (`&T`) takes the place or the
  reference it already is, a mutably lent one `&mut`, and only a moved
  position or a value one (an operator operand, an owned field) takes a
  value — a Copy scalar by copy, anything else by `.clone()` unless the IR
  marks the read a consume, in which case it is the move. So a clone happens
  only where the program needs a value of its own and did not give one up
  ([copy-opt-in]).
  * An argument to an **intrinsic** takes its mode from the *intrinsic's own
    declaration*, through the pass mode the IR gives that parameter, not from
    the position the call sits in: kept is a read, consumed (`=> !elem`) a
    value, and a `Mut` parameter is the place the lowering writes through.
  * [rs-narrow-mut] `x!` in a **mutably lent** position reaches the payload
    mutably (`&mut x` … `as_mut().unwrap()` … `&mut *__some`), so
    `add(xs!, 3)` appends to `xs` itself. (A clone there would compile and
    append to the clone, the [backend-never-wrong] failure this rule exists
    for.)
  * A **narrowed** read obeys the position the same way
    ([rs-union-enums]): the narrowed local is a reference into the storage
    (`let mut __narrowed_4 = b.label.as_ref().unwrap();`), passed as it stands
    to a `&T` parameter (`crate::len_of(__narrowed_4)`) and cloned where a
    value is needed. A **Copy** payload is copied out of the representation
    (`n.unwrap()`, `*__v`), free, and a value everywhere.
  * An interpolated part is a value (`format!` arguments are borrowed by the
    macro either way): a native part is the place, a non-native one the
    `to_str` call [rs-interp-to-str].

## Output layout

* [rs-crate] The output is a single-binary Rust crate compiled straight
  from the emitted files (`rustc --edition 2021 main.rs`). The module
  declaring `fn main` becomes the crate root; it starts with the crate
  attributes (`#![allow(...)]` for cosmetic lints the generator does not
  fight: `non_snake_case`, `unused_parens`, `unused_mut`, `unused_braces`,
  ...) and one `#[path = "..."] pub mod <mangled>;` declaration per other
  emitted file.
  For a library compile (no `main`), a synthetic `lib.rs` carries the
  attributes and mod declarations.
  * A Salvo module `core.console` emits to `core/console.rs` and mounts
    as `pub mod core_console` (path parts joined with `_`): Rust module
    paths are flat, and generated code names its items
    `crate::core_console::println`.
  * The generated union enums live under `unions/`, mounted as
    `#[path = "unions/mod.rs"] pub mod unions;` [rs-union-enums], and the
    float-text helpers in `strings.rs` [rs-float-text], each emitted only
    when the program needs it. Iteration needs no runtime file: it emits
    inline pass drives [rs-iter-pass].
* [rs-imports] **Generated code names everything by its full path**
  (`crate::geometry::area(…)`, `crate::Fighter { … }` for the crate root), so
  no call, type or construction depends on an import. The `use` items a file
  does carry are **for host files**: a host file mounted for a module does
  `use crate::<module>::*`, and expects the names that module's declarations
  mention to be in scope through it. So each emitted module gets one
  `use crate::<mod>::<Name>;` per name its text mentions as
  `crate::<mod>::<Name>` from exactly one foreign module — not names it
  declares itself, not `__`-prefixed ones, and not the runtime modules
  (`unions`, `scheduler`, `wire`, `seq`, `strings`, `hosttime`,
  `hoststreams`) or host modules (`platform_*`). The runtime files carry no
  `use crate::…` at all. An aliased Salvo import changes nothing in the
  output: a call names the declaration's path, not the alias.
  Intrinsic lowerings name everything by absolute path [intrinsic-fn].
  * Every emitted item (fn, struct, trait, impl fn, enum) is `pub`, and so
    are a struct's fields, so any module can reach them by path. A
    handler's fields stay private (except `__mailbox_capacity`,
    [rs-mailbox]): only its own impls read them.
* [rs-entry] `fn main() [use]` emits as Rust `pub fn main()` with no effect
  parameters. The CLI reports the crate-root file as the entry point.
* [backend-companion] Companion `.rs` files are copied verbatim and
  mounted like generated modules. A companion must not collide with a
  generated file.

## Type mappings

* [safe-call] [rs-safe-call] `receiver?.member` is a block that tests the
  receiver once and wraps the member read:

  ```rust
  {
      let mut __safe_1: Option<&crate::P> = maybe.clone();
      if __safe_1.is_none() {
          None
      } else {
          let mut __some_2 = __safe_1.unwrap();
          Some(__some_2.age.clone())
      }
  }
  ```

  The `Some(..)` is **omitted** when the member is already optional, or the
  result would be an `Option<Option<T>>` the declared type does not have — an
  E0308 rustc caught, which Kotlin never saw, having no wrapper to double.
* [elvis] [rs-elvis] `subject ?: rhs` is the same block with the right side in
  the absent branch: `{ let mut __elv_3 = &n; if __elv_3.is_none() { 7i32 }
  else { let mut __some_4 = __elv_3.unwrap(); __some_4 } }` — the subject
  evaluated once, bound by reference when it is a place and by value when it
  is a call's result, and unwrapped as a narrowed read [rs-read-mode]. An
  `if` rather than `unwrap_or_else` because the right side may be an
  **escape**: a `return` inside a closure returns from the closure
  [expr-escape].
* [type-basic] Internal types map natively: `Str`→`String`, `Int`→`i32`,
  `Long`→`i64`, `Float`→`f32`, `Double`→`f64`, `Bool`→`bool`,
  `Char`→`char`, `Byte`→`u8`, `None`→`()`, `Never`→`()` (a platform
  signature's result is the exception, [rs-platform-never]). (The language
  docs' original `u64` for `Long` was a spec bug — `Long` is signed; fixed
  during M8.) `Any` has no Rust mapping: a value of it reaching emission is a
  codegen error ([backend-never-wrong]).
  * [byte-value] [bytes-type] `Byte`→`u8`, and **`Bytes` is std's platform
    type over `Vec<u8>`** (`core.bytes` re-exports the host's
    `pub type Bytes = Vec<u8>;`, [rs-platform-type]): unboxed, with `Mut`
    erasing as it does for every other type here [type-canbe-mut]. The
    buffer class Kotlin has to ship ([kt-bytes]) is simply what this backend
    gets from `Vec`. `to_byte`/`to_int` are `as` casts with the **source type
    named** (`(((x) as i32) as u8)`), so a literal operand keeps its own type
    rather than taking the cast's.
* [op-promote] Rust has no mixed-width operators (`i32 + i64` is E0277),
  so a checker-recorded promotion casts the operand **as a whole**:
  `i64::wrapping_add(x, ((i32::wrapping_mul(n, 2i32)) as i64))`,
  `(((n) as i64) < x)` — the parentheses matter, since `as` binds tighter
  than every arithmetic operator. Targets are `i64` and `f64` only (widening
  goes up within a class).
* [rs-op-wrap] [op-wrap] Integer `+`/`-`/`*`/`/`/`%` on `Int`, `Long` and
  `Byte` render as `i32::wrapping_add(a, b)` (and `i64::`/`u8::`,
  `wrapping_sub`, `wrapping_mul`, `wrapping_div`, `wrapping_rem`), negation
  as `i32::wrapping_neg(x)` (a literal too: `-1` is
  `i32::wrapping_neg(1i32)`), and a step as `x = i32::wrapping_add(x, 1i32)`:
  the function form, because a method on a bare literal is E0689. A borrowed
  scalar operand is dereferenced first [rs-cmp-deref]. `bit_ushr` casts
  through the signed type before the unsigned one
  (`(u32::wrapping_shr(((x) as i32) as u32, (n) as u32) as i32)`).
* [lit-adopt] Every numeric literal renders at its **checked** type, suffixed
  (`1i32`, `1i64`, `3f64`, `0.5f32`) — an unsuffixed literal included, at its
  default type [lit-numeric] — so nothing is left to rustc's literal
  inference. [op-convert] lowers to `as` casts (`((x) as i64)`), whose
  semantics match Kotlin's `toX()` pairwise (saturating float→int,
  low-32-bits `i64`→`i32`).
* [effect-at] Erased: the IR's call names the effect instance it resolved
  to, and emission is the ordinary member dispatch through that handle.
* [fn-overload-at] [fn-rename] Both caller-side overrides of overload
  resolution are **erased**: the IR's call names the declaration, and the
  emitted path is that declaration's own name [rs-fn-mangling], so Rust has
  nothing to re-resolve.
* [rs-shadowed-call] A call that reaches past a **local of the same name**
  (`f@module(...)`, a call whose arguments do not fit an implicit of its name
  [implicit-resolve-body], an interpolation's `to_str` inside a fn with an
  implicit `to_str`) needs nothing of its own: every call to a declaration is
  spelled as a path (`crate::<mounted module>::f(...)`, `crate::f(...)` for
  the crate root), and a path never resolves to a local. Rust puts functions
  and locals in one value namespace, so a bare name would be the local
  (E0618); Kotlin needs nothing, which is why this rule is backend-prefixed.
  A call *through* a local (a fn value, an implicit) is the bare name.
* [rs-seq] std's sequence functions [seq-iterator]: the `List` fast paths
  are std functions (2026-10-04, [platform-value-type]): `map` and `reduce`
  are Salvo loops (`crate::core_seq::map__List_Fn::<i32, i32>(&xs, …)`), and
  `filter` is a platform fn (`crate::core_seq::filter_platform`, host in
  `platform/core/seq.rs`) cloning each kept element. Their fn-typed
  parameters render as `&mut dyn FnMut(&T) -> U` [rs-fn-param-convention],
  which is what gives a lambda argument its parameter types: the lambda is
  written untyped (`&mut |mut n| -> i32 { i32::wrapping_mul(*n, 2i32) }`)
  and rustc takes the types from the coercion. A **named fn** argument wraps
  in an adapter closure with typed parameters [fn-contract]
  (`&mut |__a0: &i32| crate::double(*__a0)`), since a fn item's own
  convention is by value. `runtime/seq.rs` holds only `salvo_pair_mut`
  [rs-elem-mut].
* [implicit-intrinsic] An `intrinsic fn` filling an implicit parameter is
  passed as an adapter closure whose body is the intrinsic's *lowering*
  (`&mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])` for `eq` on
  `Str`). A resolved *declared* fn's adapter forwards each argument in that
  fn's own pass mode [rs-borrows] — a kept struct parameter is `&T`, a
  consumed one is cloned out of the borrow.
* [rs-mut-str] `Str` and `Mut Str` are **both** `String`: mutability lives
  in the binding and the reference [type-canbe-mut], so a `Mut` drop
  renders nothing at all [str-drop-mut].
  * The string operations are std's **platform fns**
    (`std/platform/core/string.rs`), called through their wrappers like any
    other: `crate::core_string::size_platform(&s)`,
    `crate::core_string::index_of_platform(&hay, &ll)`,
    `crate::core_string::set_platform(&mut b, 0i32, 'H')`; a `Mut Str`
    parameter is `&mut String` and passes on as `&mut *s`.
  * String indexes are **characters**, not bytes: the host's `size` counts
    `chars()`, and `index_of`/`substr`/`set` convert. (Kotlin counts UTF-16
    code units — the divergence `size` already had.)
  * `mut_str(parts)` is a Salvo fn over `Vec<String>`
    (`crate::core_string::mut_str(vec![pa.clone()])`): its variadic
    position consumes, so a part the caller keeps is cloned into the vector
    [fn-variadic].
  * `strings.rs` holds only the float-text helpers [rs-float-text].
* [kt-none-unit]-equivalent: `None` as a return type is `()` (omitted);
  `None` as a union arm is `Option` [rs-option].
* [rs-option] `T?` maps to `Option<T>`: `None`→`None`, `is None`→
  `.is_none()`, `x!`→ the test-and-unwrap block [rs-assert-trap]. Optionals
  are *physical* in Rust, so the checker's `WrapOption` coercion emits
  `Some(code)` [type-nullable] (Kotlin ignores the same coercion).
* [type-array] `T[]` maps to `Vec<T>`; `array_of` emits `vec![...]` and
  `array_by(n, f)` an iterator-map-collect
  (`(0..(3i32)).map(|mut k: i32| -> i32 { … }).collect::<Vec<_>>()`,
  [col-by]); indexing casts the `i32` index (`v[(i) as usize]`).
* [rs-iter-pass] Iteration is **passes all the way down** [iter-protocol]:
  there is no iterator type and no runtime support module for one. A pass is a
  plain struct, `next` is a plain function, and a `for` over one is a `loop`
  that calls `next` once per turn, tests the arm, and binds the element — an
  inlined call per element, no allocation, no trait object:

  ```rust
  loop {
      let mut __step_7: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Iter_halving_Int(&mut __pass_6);
      if matches!(__step_7, crate::unions::Union2::U1(_)) {
          let mut __emitted_8 = match &__step_7 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
          let mut n: i32 = __emitted_8;
          …
      } else {
          break;
      };
  }
  ```

  A named pass is driven where it lives (`next(&mut *p)` for a `Mut`
  parameter, `next(&mut q)` for a local) and is left where the loop stopped.
  * [iter-fn] An `iter fn` is desugared before emission into that same shape: a
    struct holding what the body reads of the subject plus its `state` fields
    (`__Iter_<fn>_<Subject>`, e.g. `__Iter_halving_Int`), an `iter` that mints
    one, and the body as the `next`. The backend has no rule of its own for
    the form.
  * A `for` over a **container** calls its `iter` once before the loop
    (`let mut __pass_9: crate::__Iter_iter_Bag<'_> = crate::iter(&bag);`)
    and drives the result [iter-mint]; the intrinsic containers keep their
    native loop instead [iter-for-native]: `for mut n in xs.iter().copied()`
    over Copy elements, `for mut f in roster.iter()` over a borrowed list,
    `.into_iter()` when the loop consumes it, `.iter().cloned()` when an owned
    element is needed from a kept one, `s.chars()` over a `Str`.
  * An **effectful `next`** takes its handles as leading arguments, threaded
    into every turn of the loop from the scope the `for` is written in
    (`crate::next__Iter_fibs_Int(&__handle_2, &mut __pass_12)`, [fn-effects]).
  * [linear-group] A pass with a `close` is **not** released by the loop: the
    program's own `close(p)` after the loop is the discharge (no implicit
    discharge sites, [iter-drive-in-place]), so the loop emits nothing extra.
  * [rs-fn-field] A fn-typed **field** is `std::sync::Arc<dyn Fn… + Send +
    Sync>`, which is what lets a composed pass store its source's `next`; a fn
    that stores a callback therefore takes it owned, as
    `impl Fn(…) + Send + Sync + 'static`.

* [rs-implicit-turbofish] A **generic call that fills implicit parameters**
  spells out its type arguments
  (`crate::core_seq::map_to::<Vec<i32>, crate::Countdown, i32, i32>(…)`).
  Each implicit arrives as an adapter *closure* whose parameter types Rust
  infers from the callee's bound, so with the callee's generics still open
  there is nothing to infer them from and inference stalls (E0282) on
  closures waiting for the answer the call itself would have given.
  * An implicit's fn type renders its parameters through the **contract**
    [fn-contract]: a kept `Mut` position is `&mut T`, a kept non-Copy one
    `&T`, a consumed or Copy one by value
    (`add: &mut dyn FnMut(&mut D, U)`).
  * An implicit a callee **keeps** follows the same convention a written stored
    callback does [rs-fn-field]: owned `impl Fn(…) + Send + Sync + 'static` in
    the signature, `Arc`-held once stored, and passed by a `move` adapter at
    the call site.
* [rs-none-unit] `None` is Rust's `()`, and a fn returning it has no return
  type — so `return None` emits a **bare** `return;`. The test is the *fn's*
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
  implicit group spreads (`?Yield<It, T>`'s `next`):
  `pub fn taking<It: Clone, T: Clone>(mut it: It, mut n: i32, next: impl Fn(&mut It) -> … + Send + Sync + 'static) -> crate::core_seq::Taking<It, T>`.
* [rs-fn-field] A **function in a struct field** is
  `std::sync::Arc<dyn Fn(A) -> R + Send + Sync>`:
  * `Fn`, not `FnMut`, because it is reached through a shared `Arc`; `Send +
    Sync` because a struct may cross a spawn, and a Salvo lambda captures by
    value, so every value the program can build satisfies them;
  * a **store** wraps in `std::sync::Arc::new(…)`, in a struct literal and in
    an inlined default alike — a named fn through a `move` adapter
    (`f: std::sync::Arc::new(move |__a0: i32| crate::twice(__a0))`), a
    stored parameter as it is (`std::sync::Arc::new(f)`);
  * the struct gets `#[derive(Clone)]` and a **hand-written `Debug`** that
    prints the callback as `<fn>` (`.field("f", &"<fn>")`), since `dyn Fn`
    has no `Debug`;
  * a read is an ordinary place read (`let mut g = d.f.clone();`, an `Arc`
    bump), and calling the value still needs a local first (`let g = h.f`
    then `g(e)`) — `h.f(e)` is dot-notation for `f(h, e)` [fn-dot], which
    is a language rule, not a backend one;
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
  Reads clone like any other projection in a value position, and a narrowed
  element unwraps as a field does
  (`let mut __narrowed_3 = maybe.0.as_ref().unwrap();`, [flow-place]
  [rs-option]).
* [type-str] Strings are `String` (owned). Plain string literals emit
  `String::from("...")`; interpolation emits `format!("{}...", args)`, and a
  string of literal parts only folds to one `String::from`.
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
* [type-alias] Aliases are expanded before rendering: the emitter renders the
  IR's types through the alias table (`unalias`), so an alias never reaches
  the output by name.
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
  no per-type `Mut` mapping at all: mutability is expressed in bindings and
  references (`let mut`, `&mut`) [rs-borrows], not in the type. Struct `Mut`
  works the same way (all struct fields are plain fields; assignability is
  enforced by the checker).

## Ownership and borrowing [rs-borrows]

The central design (per docs/language/Deductions-and-Ownership.md): the
checker's deductions ([deduce-syntax] [deduce-infer]) are the ownership
contract, and the IR carries them as **pass modes** on every parameter and
argument and **consume marks** on every read [rs-ir]. Salvo source has no
references; the Rust backend spells the modes mechanically:

* **Parameter modes** (decided by core, [param-mode]; this is their Rust
  spelling):
  * *moved* (`=> !p`, or inferred so) → **by value**, bound `mut p: T`. The
    checker guarantees the caller no longer uses the argument, so the move
    is always legal.
  * *lent mutably* (kept, and the type carries `Mut`, or its elements do:
    `List<Mut T>` lends mutable handles) → **`&mut T`**
    (`pub fn fill(list: &mut Vec<i32>, mut n: i32)`).
  * *lent* (kept without `Mut`) → **`&T`** (`pub fn read(list: &Vec<i32>) -> i32`).
  * **Copy exception:** parameters of Copy scalar types (`Int`, `Long`,
    `Float`, `Double`, `Bool`, `Char`, `Byte`, an actor `Addr`) are always
    passed by value, qualified or not (`Idx(list) Int` is `mut i: i32`) — a
    borrow would be noise, and copying preserves the semantics of both kept
    and moved deductions.
  * Variadic parameters are owned `Vec<T>` (the caller assembles a fresh
    vector at the call site) [fn-variadic].
  * Qualifier `qualifies` fns and handler members follow the same modes
    (`pub fn Surname__Person_qualifies(person: &crate::Person) -> bool`,
    `pub fn Positive__Int_qualifies(mut int: i32) -> bool`); effect members
    follow their written clause [rs-effects].
* **Argument rendering.** Each argument follows its parameter's mode:
  * moved position → a value (see below): an owned local moves (`crate::consume(items)`);
  * `&T` position → `&place` (`crate::read(&items)`); a binding that already
    is a reference passes as it is (`crate::core_list::size_platform::<i32>(xs)`),
    and a `&mut` one reborrows shared (`&*squad`);
  * `&mut T` position → `&mut place` (`crate::fill(&mut items, 3i32)`); a
    `&mut` binding reborrows (`&mut *xs`);
  * a non-place argument in a borrowed position is borrowed where it stands
    (`&String::from("good")`, `&format!(…)`), or bound first when the result
    keeps the borrow [rs-loop-temp].
* **Values.** A position that needs a value of its own gets one:
  * an owned local moves where the IR marks the read a consume, and clones
    otherwise (`(s).clone()` for a `copy`, [rs-copy]); a Copy scalar copies;
  * a field or element read of a non-Copy type clones
    (`(ada.name).clone()`) — moving out of a place behind a (possible)
    reference is not legal. Exception [fate-move-mode]: a consuming read of
    an owned root's projection is a real partial move (`crate::eat(p.tags);`,
    `let mut name: String = person.name;`);
  * call results, literals, and constructed values are already owned.
  * Assignment targets, `is` subjects, and borrowed positions use the place
    itself (no clone).
  * **Borrow-mode bindings borrow [rs-borrow-locals]:** a `let` of a *pure
    place* (a local, or a field/tuple-index chain off one) that is never
    reassigned, mutated through, captured or consumed binds a reference:
    `let mut n = &person.name;`, `let mut zs = &xs;` (`&mut place` when the
    binding is in the place's alias group and the program mutates through
    it). Reads thread through it like any reference binding: bare in a
    borrowed position, `*n` for a Copy scalar, cloned where a value is
    needed. A `for` over a pure place iterates *by reference*
    (`for mut person in persons.iter()`) and the loop variable is a
    reference binding — no collection clone. Everything else (reassigned
    names, coerced values, union payloads) takes its own value, which is
    sound by restriction: the checker poisons a binding before any write
    the borrow would outlive [fate-poison], so legality aligns with NLL.
    * **Field-disjoint borrows need nothing extra**
      [fate-field-disjoint]: the checker lets `let n = p.name` stay live
      across a mutation of `p.tags`, and that emits as a real borrow held
      across `&mut p.tags` (`let mut n = &p.name;` …
      `crate::core_list::add_platform::<String>(&mut p.tags, …)`), which
      rustc accepts because they are disjoint fields of one local. The
      **move** half matches too [fate-partial-move]: a moved projection
      emits as a real partial move of the field and a later reassignment as
      rustc's reinitialization.
  * **Lambdas emit plain (borrowing) closures [fate-lambda]**, passed as
    `&mut |…| …` to a lent fn parameter: captures are rustc borrow-captures,
    which alias — the same semantics as Kotlin's lexical capture. A closure
    is poisoned by a root mutation (its borrows end before the mutation
    under NLL) and mutated captures are consumed at creation. A closure
    filling a `once` or a stored position is a `move` closure
    (`crate::once_it(move |mut a: i32| -> i32 { a })`) [once-fn]
    [rs-fn-field].
  * **Move-mode bindings move [fate-move-mode]:** a binding whose read the
    IR marks consuming emits the place itself — a real move, partial for
    projections — and a consuming `for` iterates *by value*
    (`for mut person in persons.into_iter()`). This is what makes inferred
    consuming pipelines zero-clone end to end.
* **Local bindings.** Every `let` emits `let mut` with the type written
  (`let mut n: i32 = …;`; the crate-root `#![allow(unused_mut)]` silences
  the cosmetic lint): Salvo mutability (assignment, `++`, `Mut` methods,
  `&mut` argument positions) is otherwise undecidable locally. A borrow-mode
  local, a fn value and a binding whose type holds a nested projection leave
  the type to rustc.
* **Lifetimes.** Struct fields are owned and — with the exceptions of
  [rs-proj] — results are owned, so functions are lifetime-elision-friendly.
  A derived-return fn returns `&T` / `Option<&T>` [readonly-return];
  elision covers the single-reference-parameter case
  (`pub fn window(roster: &Vec<crate::Fighter>) -> crate::Window<'_>`), and
  with more reference parameters a `'a` is named on the lent parameters and
  the return
  (`pub fn named<'a>(roster: &'a Vec<crate::Fighter>, name: &String) -> Option<&'a crate::Fighter>`).
  Return values render as borrows (`return Some(f);` for a by-reference loop
  variable, the reference itself for an already-`&` binding,
  pass-through for forwarded derived calls).
* **Generic bounds.** A generic parameter gets a `Clone` bound
  (`<T: Clone>`), since a value position may clone a value of generic type —
  except on a `canbe Linear` parameter and on a std platform wrapper
  (`pub fn get_platform<T>(…)`), which never clones. Structs
  `#[derive(Clone, Debug, PartialEq)]` — `PartialEq` unconditionally, since
  it costs nothing where Salvo refuses `==` anyway [col-equality] — and
  nothing else: no `Eq`, `Hash` or `Ord`, since a `List<Point>` or
  `(Int, Point)` key goes through Salvo's list and tuple `eq`/`hash`/`cmp`
  to the element's Salvo fn [col-hashed-ordered]. A struct with a fn-typed
  field derives only `Clone`: `Arc<dyn Fn>` has neither `Debug` nor
  equality [rs-fn-field]. Generated union enums derive
  `Clone, Debug, PartialEq` (conditional on their payloads, as derives are),
  so a struct holding one can derive its own.
* Deliberate simplicity, accepted costs: a lent argument is never moved even
  when it would be its last use (a clone happens instead where a value is
  needed); rustc's borrow checker remains the final authority — a program
  that emits but does not borrow-check is a compiler bug, not a user error.

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

* [rs-qualified-scalar] A **qualified scalar** parameter (`index: Idx(list)
  Int`) is a Copy scalar like any other: passed by value (`mut index: i32`),
  a `platform fn` wrapper hands it to the host as it is, and a dependent
  qualifier's predicate takes it by value too
  (`crate::core_index::Idx__Int_qualifies(i, &squad, …)`). A slot argument
  that is a `&mut` binding reborrows as shared (`&*heap`), which keeps a
  generic slot (`Idx`'s `c: C`) from binding to the `&mut` itself
  ([col-idx]).
* [readonly-return] A wholesale projection returns `&T`, `Option<&T>` or
  `Union2<&T, Finished>`. One reference parameter: lifetime elision. More:
  `'a` is named on **every** source parameter and the return
  (`pub fn longest<'a>(a: &'a String, b: &'a String) -> &'a String`), and
  only there — a lent parameter the result does not borrow keeps an elided
  lifetime. Implicit parameters count: one rendered `&mut dyn FnMut` is a
  second reference, so `top(r: Ranked<T>(?cmp)) -> (proj(r) T)?`, whose
  binder is captured from `r`'s type [cmp-binder], names `'a`. A returned
  projection of a pass parameter that is itself a borrowing struct names the
  *struct's* source lifetime instead
  (`pub fn peek<'a>(w: &crate::Window<'a>) -> Option<&'a crate::Fighter>`,
  [rs-proj-struct]), so the reborrow of the parameter is free for the next
  turn.
* [rs-opt-borrow] A local bound from an optional projection (`let h =
  first(xs)`) holds `Option<&T>`; a later narrowing unwraps the reference
  (`let mut head_3 = head.unwrap();`) rather than cloning the reference
  itself, and a value position clones through it (`(head_3).clone()`).
  * **A Copy scalar's optional borrow is copied at the source**: the call is
    `get_platform::<i32>(&xs, 0i32).copied()`, so the local, the `?:` subject
    and the `is` subject are an owned `Option<i32>` and unwrap with
    `.unwrap()`. Likewise an optional borrow of a Copy scalar flowing into an
    owned `T?` (an argument to an `Int?` parameter, an annotated
    `let v: Int? = get(xs, i)`, a `copy(get(xs, i))`) is `(…).copied()` /
    `.cloned()`.
  * [elvis] The same rule for the **`?:` read** and for an `is` binding over
    an optional borrow: a non-place subject is bound first
    (`let mut __subject_13: Option<&String> = …;` before the `if`,
    [rs-is-hoist]), the binding is `let mut s = __subject_13.unwrap();`, and a
    value position clones through it (`(s).clone()`); a `proj`-typed result
    keeps the borrow (`let mut h: &String = { let mut __elv_14: Option<&String> = …; … __some_17 };`).
  * **A borrow *of* an optional** (`&Option<T>`, the total `get` over a
    `List<T?>`, checker type `proj(xs) T?`) is narrowed through
    `s.as_ref().unwrap()`, a `&T`.
  * [proj-type] [lambda-view] An unwrap whose own checker type is a
    projection stays the reference — no clone, no deref: a lambda tail
    `get(all, i)!` typed `proj Str` yields `&String` into the closure's
    return (`&mut |mut i| -> &String { … }`, `Vec<&String>` at the call),
    and an interpolated `first(names)!` displays through the reference. Any
    position that truly needs ownership was checker-refused without `copy`
    before emission.
  * A Copy scalar read out of a reference binding (a lambda parameter
    under the `FnMut(&T)` convention [rs-fn-param-convention]) renders as a
    *value* wherever a value is needed (`*n`, `*__a1`): in operators, in
    casts, and in an argument to a by-value parameter.
  * An intrinsic argument whose parameter the declaration **consumes**
    (`=> !value` — `send`, `discard`, `add`, `insert_sorted_by`, `put`,
    `reduce`'s seed) is a **value**, not a place: the lowering takes
    ownership, so the same rendering an ordinary consuming call gets applies
    (a real partial move stays a move [fate-move-mode], a read the caller
    keeps clones). The checker refuses the shape that would need a move out
    of a handler's own field ([effect-state-store]'s read direction).
* [rs-proj-struct] A struct with a `proj` field — or an owned field whose
  type has one, transitively — is a **borrowing struct**:
  `pub struct ListYield<'s, T> { pub items: &'s Vec<T>, pub at: i32 }`, with
  `<'s>` on owned view-typed fields; every mention elides
  (`crate::core_list::ListYield<'_, T>`); a struct literal borrows into its
  `proj` fields (`crate::__Iter_iter_Bag { items: &bag.items, at: 0i32 }`,
  or the reference itself when the source already is one:
  `crate::Window { roster: roster, at: 0i32 }`); it is returned *by value*
  (the struct carries the lifetime, no `&` wraps it).
* [rs-proj-lends] The lifetime a view carries reaches the parameters it
  borrows [proj-infer]: with one reference parameter elision ties them;
  with more, `'a` is named on every lent parameter the result borrows and
  on the return. A lent parameter that is **itself a borrowing struct**
  defeats elision even alone — `p: &mut ListEnumYield<'_, T>` has two
  input lifetimes — so `'a` is named there too, and it tags the struct's
  **inner** (source) lifetime, never the `&mut`: the returned view borrows
  the iterator's *source*, so the reborrow of `p` stays free for the next
  turn, exactly as [rs-proj-struct] ties a derived return. A **lent
  implicit position** (`?iter: (c: C) -> Mut It holds proj(c)`) renders
  `&'c C` under a lifetime named on the enclosing fn's kept parameter — the
  result's type (`It`) is fixed at the call site, so the borrow it holds
  cannot be a fresh per-call one — and **each** such lent parameter gets a
  lifetime of its own:
  `pub fn count<'k, 'c, C: Clone, K: Clone, It: Clone, T: Clone>(c: &'c C, k: &'k K, iter: &mut dyn FnMut(&'c C, &'k K) -> It, …)`.
  The enclosing fn must keep the parameter (a consumed one has nothing a
  view could outlive — reported). Re-pointing entries
  (`v.items: proj(other)`) tie a lifetime on the target struct and the
  source parameters, and the assignment renders as a borrow.
* [rs-proj-arm] A union with a `proj` arm is an ordinary instantiation of
  the shared enum with a reference arm (`Union2<&'s T, Finished>`). At a
  call filling `?Yield<It, T>` from a borrowing `next`, the element generic
  is **retagged** to `&T` in the turbofish
  (`crate::core_seq::filter::<crate::core_list::ListYield<'_, crate::Fighter>, &crate::Fighter>(…)`)
  — unless the substituted type already carries the projection
  (`T = proj Str` renders `&String` on its own [proj-type]). User callbacks
  at a retagged position arrive one reference deeper and peel it
  (`let f = *f;` at the top of a lambda); resolved implicit adapters clone a
  retagged position out where the callee owns it, and `copy` at a retagged
  position is the identity. A concrete Copy element (`?Yield<It, Int>`) is
  copied out by a match adapter instead:
  `&mut |__a0: &mut crate::core_list::ListYield<'_, i32>| (match crate::core_list::next__ListYield(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) })`.
  A value of a `proj`-arm union flowing into a position written as the
  owned union is adapted arm by arm the same way for Copy payloads, and is
  a codegen error otherwise (a hidden clone this backend refuses).
* [rs-elem-mut] [ref-handle] **Mutable element handles** (P-3 + P-9, user
  decisions 2026-09-24) are **positions**, never a bound `&mut`:
  * A **statement-scoped** handle — `bump(at(es, 0)!)` in a `&mut`
    position — materializes the element for that one call: the mint's path
    is bound, then walked into a borrow,
    `{ let __h1: Option<usize> = crate::core_list::at__List_Int(&es, 0i32); … &mut es[__h2] }`
    inside the `!` block [rs-assert-trap]. The read borrow of the search
    ends before the write borrow begins, and the `&mut` lives exactly as
    long as the call.
  * A **bound** handle is its path [rs-path]:
    `let __h2: usize = crate::core_list::at__List_Int(&squad, 0i32).expect("salvo: value is absent at main:231:16");`
    — the presence check traps at the mint, matching Kotlin's `!!` timing —
    and every use re-materializes the place: `squad[__h2].hp = …`,
    `crate::bump(&mut xs[__h1]);`, `squad[__h3].name.clone()` in a value
    position. Deliberately not a bound `&mut`: Salvo's poison discipline
    permits reads of the container between uses of the handle
    (`let mut n: i32 = crate::core_list::size_platform::<crate::Fighter>(&squad);`
    between two writes through `__h2`), which a live `&mut` binding would make
    E0502 — the same alignment argument as [rs-borrow-locals], resolved the
    other way. The total mint answers `usize` directly
    (`let __h11: usize = crate::core_list::at__List_IdxInt(&squad, i);`).
  * A kept parameter whose **elements** carry `Mut` (`List<Mut T>`,
    `Mut T[]`) renders `&mut Vec<T>`: the container lends mutable handles, so
    the write must reach the caller's storage through it even though no
    structural mutation is permitted.
  * A **proven-distinct pair in one call** ([elem-distinct], two bound
    handles under a `NotSame` claim [ref-notsame]) splits where the two
    paths diverge [rs-path] and the call takes the two `&mut` halves: one
    `crate::seq::salvo_pair_mut` (a `split_at_mut`, [rs-runtime-source]) at a
    list position, `Map::pair_mut` at a map slot, plain disjoint `&mut`s at
    different fields; paths that never diverge trap (the claim ruled it out):
    `{ let (__pm9, __pm10) = if __h5 != __h6 { let (__u7, __w8) = crate::seq::salvo_pair_mut(&mut squad[..], __h5, __h6).expect("salvo: value is absent"); (&mut (*__u7), &mut (*__w8)) } else { panic!("salvo: two handles to one element") }; crate::duel(__pm9, __pm10) };`.
    The form is a block expression, so it stands in a value position as
    well as a statement.
  * `NotSame`'s predicate `same(a, b)` is `std::ptr::eq` on the two
    materialized references.
  * Kotlin needs none of this — objects alias natively, so the handle is the
    element reference.
* [rs-cmp-deref] **A borrowed Copy scalar is copied out where an operator
  takes it**: Rust implements neither `&i32 == i32` nor `&i32 < i32`
  (E0277), and `i32::wrapping_add(&i32, …)` has no impl either, so a lending
  call's scalar result (the total `get` at an `Idx`/`KeyOf` claim,
  `first(NonEmpty)`) is dereferenced as an operand:
  `if (((*crate::core_list::get::<i32>(&xs, i))) == (20i32))`,
  `i32::wrapping_add((*crate::core_list::get::<i32>(&xs, i)), 1i32)`.
  Non-Copy operands are not touched (a deref there would move out of a
  borrow). Kotlin has no references and needs nothing.
* [rs-path] **A handle is a storage path** (user decisions 2026-10-10,
  GROUP_BORROWING.md Part 9; `ir_emit/paths.rs`): a `ref(c)` handle is the
  path from its container — field and tuple steps, list/deque/array
  indices, map *slots* — computed once at the mint by running the mint's
  body, and walked at every use. The IR names each handle's container type
  on its `ref` qualifier ([ref-handle], `salvo_ir::build::refs`), and the
  path's type belongs to the (container type, element type) pair, read off
  the type definitions:
  * **one way** from container to element: the tuple of its dynamic
    positions — `usize` for a list or a map (slot), `()` for none — and a
    use is a plain place (`squad[__h2]`, `a.b.c[__h1]`, `l.teams[__h3].roster[__q4]`);
  * **several ways** (an accessor that may answer from different fields): a
    generated `#[derive(Clone, Copy, PartialEq)] enum __Path_<C>__<E>`, one
    variant per way holding its positions, declared in the container's
    module; a use walks it with a `match` (`(*match __h1 { V0(s, i) => &mut l.teams[s].roster[i], V1(i) => &mut l.bench[i] })`,
    `&` for a read);
  * a container that holds the element type in itself (a recursive type) or
    is a type parameter has no path yet: a loud error naming it.
* [rs-loc] **Locator-specialized lending** (④a, 2026-09-24; re-based on
  [rs-path] 2026-10-10): a **mint** — a fn whose result is a `ref(c) Mut`
  handle — renders **only** as its path fn, under its own name: no
  `&mut`-answering face beside it (`pub fn wounded(squad: &Vec<crate::Fighter>) -> Option<usize>`;
  std's `at` overloads mangle as `at__List_Int`/`at__List_IdxInt`). A
  *reader* (a `proj`-returning fn, e.g. `get`) used as a handle keeps its
  natural face and gains a **demand-driven locator variant**, `{name}__loc`.
  * Either answers **position data** — the path type [rs-path], optional
    exactly where the result is, so `!` keeps its message and timing. Its
    lent parameters drop to *read* mode (the search borrows nothing
    mutably), and it carries **no lifetimes** — a locator is owned data,
    which is what lets it pass through closures. A **platform** fn's variant
    finds the host's borrow by address
    (`….map(|__x| list.iter().position(|__e| std::ptr::eq(__e, __x)).expect("salvo: a borrow outside its container"))`),
    except `List`/`Deque`'s `get`, whose position is the index under a bounds
    test, the total `get_at`, which is the index, and `Map`'s `get`, whose
    position is the entry's slot.
  * Body transform: return-path forwards take their callees' path fns or
    `__loc` variants (demand closes transitively); a derived-return intrinsic takes
    its **locator form**; an intrinsic without one, or a return shape beyond
    the plain and optional element lend, is a reported error, never a silent
    read lowering [backend-never-wrong].
  * **Search loops**: inside a locator variant, a `for` directly over a list
    lowers to an *indexed* loop whose element binding is a borrow of the
    indexed place, so `return e` answers the found **position**:
    `for __li1 in 0..squad.len() { let f = &squad[__li1]; if (f.hp < 10i32) { return Some(__li1); }; }`.
  * **Fn values**: a fn type whose return is a wholesale mutable lend
    renders as a **locator closure** —
    `at: &mut dyn FnMut(&Vec<crate::Entity>, &L) -> Option<usize>`,
    read-mode parameters — and a lambda filling such a position emits in
    locator mode (`&mut |mut c, mut k| -> Option<usize> { crate::core_list::at__List_Int(c, k) }`),
    a named mint through its path fn
    (`&mut |__a0: &Vec<crate::Fighter>, __a1: &i32| crate::core_list::at__List_Int(__a0, *__a1)`).
    The call materializes the borrow:
    `match at(&*squad, l) { Some(__l1) => Some(&mut squad[__l1]), None => None }`.
    This is what lets a mutable handle cross a closure boundary at all; a
    `&mut`-returning `FnMut` would tie the borrow to the closure. The
    `?at`/`Ref` idiom [col-locate] rides it, with implicit positions
    rendered the same way.
  * **Effect members** that lend mutably are mints too, and render the same
    way (2026-10-10): **only** the path face, under the member's own name,
    on both traits and the handle —
    `fn lease(&mut self, es: &Vec<crate::Entity>) -> Option<usize>;` —
    the anchor lent for reading, the answer the path type, no lifetimes.
    The handle dispatches it through either arm, the lock's guard included:
    the result is owned data, never a borrow of handler state. Every call
    site goes through the mint: a handle bound from the member
    (`let e = lease(es)!`) is the path `lender.lease(&*es)`, walked at each
    use, and a call in a value position walks it once. A handler body
    delegating to a mint (`return pick(t, i)`) answers that mint's path. A
    `platform handler` cannot implement a member returning a `ref` (a
    checker error, [ref-handle]); an actor's members cannot return one.
  * **Anchored handle parameters** ([ref-anchor], 2026-10-08, replacing
    `canbe`'s covered positions): a parameter `a: ref(c) Mut T` renders as a
    **`usize` position in its container parameter**, which the callee
    already takes —
    `pub fn strike(c: &mut Vec<crate::Fighter>, __c1: usize, __c2: usize)`
    — and the body indexes `c[__cN]`. The call site computes the positions
    first (a bound handle's own path, or the mint's) and passes
    the container `&mut` once:
    `{ let __q7 = crate::core_list::at__List_Int(&squad, i).expect(…); let __c8 = __q7; …; crate::strike(&mut squad, __c8, __c10) };`.
    The parameter's type is the path type [rs-path] (`a: ref(l) Mut Player`
    over a `League` takes `__c1: crate::__Path_League__Player`), and an
    argument's path is taken relative to the container argument.
    Two `&mut` into one container cannot coexist, which is why sharing a
    container changes the *representation* rather than relaxing a check;
    aliasing is then exact (one storage), so the call behaves identically
    to Kotlin's native aliasing — including the case where both handles are
    the same element. The container may be a field path at the call:
    `crate::rotate(&mut team.members, __c11, __c12)`. Reported, loudly: an
    anchored argument whose position cannot be computed.
  * **Maps** (2026-10-08): `core.map`'s `at` is the by-key mint, and a map
    position is its entry's **slot** in the insertion-ordered slab
    (`platform_core_map::slot_of`; `Map` implements `Index<usize>` /
    `IndexMut<usize>` over slots), so `get`'s locator form is
    `slot_of(map, key, hash, eq)` and every list rendering above applies
    unchanged — a handle chains through a map (`at(at(m, k)!, i)!`).
    Slots are stable until an entry is added or removed, which poisons the
    handles [fate-poison].
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
  `Display`** (built 2026-09-25). Salvo's rule is Kotlin's [interp-float],
  which Rust's `Display` matches in neither respect: it writes the number out
  in full (`100000000000000000000` for `1.0E20`) and drops the `.0` (`2` for
  `2.0`). `crate::strings::salvo_f64_text` and `salvo_f32_text` rearrange
  `{:e}`'s output — the same shortest round-tripping digits Kotlin's
  `toString` chooses, so only the arrangement differs — into the plain window
  or the scientific form, and answer `NaN` / `Infinity` / `-Infinity` for the
  specials. They take anything that borrows the float (`Borrow<f64>`).
  * Verified against Kotlin's own output on 33 values, including both
    threshold boundaries, the denormal minimum, `MAX`, `1e300`, `1e-300` and
    the f32 cases.
  * **One site**: the `to_str` intrinsic of `Double`/`Float`, which every
    path to text reaches — an interpolated part
    (`format!("{}", crate::strings::salvo_f64_text(d))`), and a container's
    or a struct's `to_str`, which are Salvo fns handed the element's `to_str`
    (`crate::core_list::to_str::<f64>(&xs, &mut |__a0: &f64| crate::strings::salvo_f64_text(*__a0))`).
* [rs-loop-temp] **A borrow of a temporary that outlives its statement is
  hoisted.** A `for` over a pass *binds* the pass, and a `let` can keep a
  borrow (a view, an optional projection), so a Rust temporary the value
  borrows would die at the end of the statement it was written in (E0716) —
  while the language allows the shape deliberately ("a view of a temporary
  may be *used* within its statement", [proj-anywhere], and a `for` is that
  use) and Kotlin's reference needs nothing. So the temporaries are bound to
  locals in front, which is rustc's own suggestion:
  `let mut __tmp1 = vec![1i32, 2i32];`
  `let mut __pass_3: crate::core_list::ListYield<'_, i32> = crate::core_list::iter::<i32>(&__tmp1);`.
  * **Which temporaries**: every argument of the call that is **lent and not
    a place**, when the call's result holds a borrow. An owned (consumed)
    position needs nothing — a moved value is not borrowed from anywhere —
    and a native container loop needs no hoist at all (the temporary lives
    to the end of the `for` statement there, which includes the body).
* [rs-mut-arg-hoist] **A read before a mutation, in one expression** (built
  2026-09-25). Rust holds a read borrow for the whole expression it sits in,
  so a sibling that mutably borrows the same place collides with it —
  `format!("{} {}", b.n, bumped(&mut b))` and `label(&b.tag, bumped(&mut b))`
  are both E0502 — while the language only orders the two
  ([deduce-same-call]: arguments are evaluated left to right, and a read is
  not a consumption), and Kotlin runs them. So the reads are hoisted into
  `let`s in front of the expression, in the order the language already gives
  them:
  * **Interpolation**: when a part lends mutably, *every* non-literal part is
    evaluated into a temp, in order:
    `{ let __part3 = b.n; let __part4 = crate::bumped(&mut b); format!("1. {} {}", __part3, __part4) }`
    (and `{ let __part15 = crate::bumped(&mut b); let __part16 = b.n; format!(…) }`
    when the mutation comes first).
  * **Call arguments**: when an argument (or a call nested in one) lends a
    root mutably, each *other* argument that reads that root is evaluated
    into an `__argN` first — by value for a moved or Copy position, as an
    owned copy lent for a lent one:
    `{ let __arg9 = b.tag.clone(); let __arg10 = crate::bumped(&mut b); crate::label(&__arg9, __arg10) }`.
    A statement-position call whose arguments needed this is the same block
    (`{ let __arg3 = …; crate::core_console::println(&__handle_2, &__arg3) };`).
  * **What it will not do**: a read of **mutable data** before a later
    argument mutates it cannot be copied out of the way, because a snapshot
    on Rust against a live handle on Kotlin is exactly the divergence the
    rule exists to prevent. That shape is reported instead, naming the two
    things the program can say — `copy(place)` for the snapshot, or the
    mutating call in a statement of its own [backend-never-wrong].
  * **Cut**: the modes come from the callee's declaration, so a call through
    an **effect member or a fn value** plans nothing and keeps whatever rustc
    makes of it. Covered and proven-pair calls plan nothing either — they
    render their own positions and preamble, and leave no read borrow
    standing.

## Unions [rs-union-enums]

* [kt-union-wrappers]-equivalent: wrapper unions emit as generated
  enums, one file per arity (`unions/unionN.rs`) under a `unions/mod.rs`
  that mounts and re-exports them (as Kotlin's; a host project's runtime
  copy stays one `unions.rs`):
  `pub enum UnionN<T1..TN> { U1(T1), .., UN(TN) }` with
  `#[derive(Clone, Debug, PartialEq)]`, per-arm accessor methods
  (`pub fn u1(&self) -> &T1`, `pub fn u1_mut(&mut self) -> &mut T1`,
  panicking on the wrong arm) for host code — generated code never calls
  them — a `Display` impl (bounded on every arm being `Display`), and an
  `impl … crate::wire::__Wire` when the wire is mounted [rs-wire].
* [union-arm-identity] Arm indices from the IR map 1:1 onto the `Ui`
  variants (positional over the runtime union's non-`None` arms,
  qualifiers erased; literals collapse [type-literal]).
  * Wrap at boundaries: `crate::unions::UnionN::Ui(code)` — the other type
    parameters come from the annotated `let`, the parameter or the return
    type — wrapped in `Some(...)` when the target union has a `None` arm
    (`return Some(crate::unions::Union3::U2(String::from("one")));`).
  * **A narrowed read binds a local** at the top of the branch, named for the
    subject with a counter, reading the arm out of the storage by a `match`
    — a reference into it, or the value for a Copy payload:

    ```rust
    if matches!(result, crate::unions::Union2::U1(_)) {
        let mut result_1 = match &result { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        …
    } else {
        let mut result_2 = match &result { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        …
    };
    ```

    A nullable repr's pattern carries the `Some` (`Some(crate::unions::Union3::U1(__v))`).
    A narrowed **field** binds `__narrowed_N`
    (`let mut __narrowed_4 = match &h.result { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };`)
    [flow-place]. Unlike Kotlin, Rust also unwraps a `T?` narrowed to its
    value arm: `let mut n_1 = n.unwrap();` for a Copy scalar,
    `let mut s = o.as_ref().unwrap();` otherwise — there is no smart cast to
    lean on. Reads of the narrowed name inside the branch are reads of that
    local, borrowed or cloned per position [rs-read-mode].
* [rs-narrow-mut] A **mutable** use of a narrowed place binds the local
  through the mutable form instead, so it is a `&mut` *into the storage*:
  `let mut p_3 = p.as_mut().unwrap();` for a nullable repr,
  `let mut q_4 = match &mut q { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };`
  for a wrapper arm, `match &mut e { Some(crate::unions::Union2::U1(__v)) => __v, … }`
  for the two composed, `__p.inner.as_mut().unwrap()` for a field. Every
  site that mutates through the narrowing reads that binding: a `&mut`
  argument (`crate::core_list::add_platform::<i32>(&mut *a_5, 9i32)`), the
  base of an assignment target (`let mut r_5 = r.as_mut().unwrap();` then
  `r_5.at = 2i32;`), an intrinsic's `Mut` parameter, a `^` branch
  [rs-widen-shadow], a handler's state field (`match &mut self.held { … }`).
  Assigning to the narrowed *variable* itself writes its storage, not
  through the narrowing.
  * **An owned parameter binds `mut` when `Mut` is reachable by peeling an
    arm** (`pub fn eat(mut o: …)`), or the peel cannot borrow it.
  * **The one refused shape** [backend-never-wrong]: a union-typed parameter
    the frame received **borrowed** (`fn f(o: Ok Mut List<Int> | Err Str)`
    is `&Union2<…>`) whose arm is mutated. There is no `&mut` to give and
    nothing can ask for one — a written `=> o: Mut` is refused, since the
    `Mut` is the arm's claim and not the parameter's — so it is **reported**
    ("read-only here … union arm"), naming the remedy (take the payload as
    its own `Mut List<T>` parameter and check the arm at the call site).
    Kotlin compiles and mutates the caller's value here, so this is a real
    divergence closed by restriction on one side; lifting it is a deduction
    question, open in ROADMAP.md.
  * **Why it is a rule and not an optimization**: a read form under a
    mutation is silently wrong. `&mut (p.as_ref().unwrap().clone())`
    compiles, and the mutation lands on the clone — an iterator driven
    through a narrowed handle re-emitted its first element for ever, while
    Kotlin (whose smart cast *is* the storage) advanced. It is the one
    [backend-never-wrong] violation this compiler has shipped (fixed
    2026-09-10); the tests assert the clone-then-mutate shapes appear
    nowhere.
  * `is` lowering ([is-narrowing]): single arm →
    `matches!(subj, crate::unions::UnionN::Ui(_))`; all arms →
    `subj.is_some()` when nullable, `true` otherwise; `is None` →
    `subj.is_none()`; nullable wrappers test `Some(UnionN::Ui(_))` patterns.
  * Re-wraps between reprs ([let-infer]): a `match` mapping arms by type
    equality — `(match x.clone() { crate::unions::Union2::U1(__v) => crate::unions::Union3::U1(__v), …, #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") })`
    — with `None => None` when both sides are nullable.
* [when-union-subject] `when` over a subject lowers to the `if`/`else if`
  chain of its arms' tests, each branch opening with its narrowing binding as
  above; the last arm is the `else`, since the checker proved the arms
  exhaustive [when-exhaustive]. `if` is an expression, so `when`-as-value
  needs no extra lowering (`let mut w: i32 = if matches!(v, …) { … 1i32 } else if … { … } else { 4i32 };`).

## Control flow

* [while-value] [rs-loop-value] Rust `while`/`for` are statements, so a
  value-position loop lowers to a block expression with a
  `let mut __loop_N: Option<T> = None;` result local assigned by the body's
  tail and by `break value`s (assign-then-`break`:
  `__loop_10 = Some(x);` `break;`); an `else` uses a
  `let mut __ran_N: bool = false;` flag tested after the loop
  (`if !(__ran_4) { … }`). The block ends by binding its value,
  `let mut __loop_value_N = __loop_N.unwrap();` when the checked join type
  has no `None` arm, `let mut __loop_value_N = __loop_N;` when the join is
  itself optional, and answering it.
* [let-destructure] A **loop pattern** binds the element to an `__elem_N`
  local and opens the body with one binding per name, read off it:
  `for mut __elem_3 in rows.iter() { let mut k = &__elem_3.0; let mut v: i32 = __elem_3.1; … }`
  (`let mut who = &__elem_10.name;` for a struct pattern). A non-Copy part is
  a reference binding [rs-borrow-locals] — the one shape that serves an owned
  element *and* a borrowed one — and a Copy part is copied out. A native Rust
  pattern in the header cannot do both: `mut k` opts out of match
  ergonomics, so it moves out of a shared reference (`E0507`). A name the
  body **assigns to** takes a value of its own (`let mut a: i32 = __elem_14.0;`).
* [if-else-none] `if` is an expression in both languages; a missing `else`
  on a value-position `if` emits `else { None }` (the branch values carry
  `WrapOption` coercions). Statement-position branches emit their tails as
  statements, and the `if` statement ends with `;`.
* [when-condition] [rs-when-cond] Rust has no subject-less `match`, so a
  subject-less `when` lowers to the `if`/`else if`/`else` chain it is
  (`if (n < 0i32) { … } else if ((n) == (0i32)) { … } else { … }`). The
  mandatory `else` makes the chain total, so nothing needs the `else { None }`
  filler of [if-else-none] and no `unreachable!()` arm is generated.
* [loop-while-is] `while x is T n` is a `loop` that tests at the top and
  re-binds per iteration:
  `loop { if !(x.is_some()) { break; }; let mut n = x.unwrap(); … }`; a
  non-place subject is bound first, inside the loop [rs-is-hoist].
* [rs-inc-dec] Rust has neither `++` nor `--` [inc-dec], so both are
  blocks over a wrapping step [rs-op-wrap]:
  * postfix answers the old value:
    `{ let mut __old_5: i32 = i; i = i32::wrapping_add(i, 1i32); __old_5 }`
    (also the statement form, whose value is discarded);
  * prefix answers the new one: `{ i = i32::wrapping_add(i, 1i32); i }`.
* [rs-interp-to-str] A non-native interpolated value [interp-to-str] is
  wrapped in the `to_str` the checker resolved: an `intrinsic` one goes
  through its lowering (a float's [rs-float-text]), a declared one is an
  ordinary call on a borrow (`format!("{}", crate::to_str(&p))`). A derived
  struct [interp-struct] has a stamped Salvo `to_str` that appends its
  fields one by one (`format!("x: {}", value.x)`) — *not* `{:?}`, which would
  quote strings and so disagree with Kotlin. A container's is std's Salvo
  `to_str`, handed the element's
  (`crate::core_list::to_str::<String>(&names, &mut |__a0: &String| format!("{}", __a0))`).
* [qual-lift] [rs-widen-shadow] A `^` check emits the same test `is` would
  (or `true` when the qualifiers are statically present — qualifiers are
  erased, so widening is a typing act). Where it *peels a wrapper arm*, the
  branch opens with the ordinary narrowing binding of the subject
  (`let mut nested_7 = match &nested { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };`),
  a reference into the storage, so reads of the subject and any nested
  `when` see the inner value (`if matches!(nested_7, crate::unions::Union2::U1(_)) { … }`).
  * **When the peeled payload is mutated** the binding is the mutable form
    (`let mut c_9 = match &mut c { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };`),
    so a mutation inside the branch reaches the storage and survives the
    branch, as Kotlin's cast of the storage does. [rs-narrow-mut] carries
    the whole set of mutable sites and the one shape still refused.
  * A `^` test on a *projection* (`h.result is ^Ok`) tests the field in
    place (`if matches!(h.result, crate::unions::Union2::U1(_))`) and binds
    `__narrowed_N` where the branch reads it [flow-place].
  * Without the narrowing the emitted code would compile and be **wrong**: a
    nested test would scrutinize the outer wrapper, whose arm 0 is the one
    the outer test already took, making the second inner branch dead code.
* [fn-effects] [rs-fn-effect-params] A fn type's effects are **leading
  `&E` handle parameters** of the closure [rs-handle]: `(s: Str)
  [Logger] -> Str` renders as
  `&mut dyn FnMut(&crate::Logger, &String) -> String`, a lambda takes the
  handle as its first parameter (`&mut |mut __leff0, mut s| -> String { __leff0.log(…); … }`),
  and the call passes the handle first. Nothing is captured, which is what
  lets the value cross a call that borrows the same effect value.
  * Inside a lambda body an effect resolves to the lambda's *own* parameter
    (`__leff0`): a parameter shadows the enclosing fn's handle rather than
    capturing it.
  * A **named fn** passed by value gets an adapter closure taking the
    *expected* handles and forwarding the ones it declares
    (`|__e0: &crate::Logger, __a0: &String| crate::shout(__e0, __a0)`) — a
    pure fn is handed them and ignores them
    (`|mut __fx0, mut __a0| -> String { crate::plain(__a0) }`, the variance
    rule).
* [rs-exit-splice] **No code is owed at a block's exits** today, so nothing
  is spliced: the `defer` statement was removed from the language
  (2026-09-10), and a pass with a `close` is closed by the program itself
  [linear-group]. A release the program writes is ordinary code before the
  exit it guards, and a throw re-raised after it needs nothing
  (`crate::close_file(console, h);` then
  `let mut n: i32 = (match crate::parse(console, line) { … => return std::ops::ControlFlow::Break(__m) });`).
  Were the compiler to owe exit code again, Rust has no `finally` and a
  `Drop` guard cannot stand in (a release consumes its handle, so the guard
  would have to own it for the rest of the block): it would be emitted at
  every exit. The known divergence from Kotlin's `finally` lowering stands
  for such code (accepted, user decision 2026-09-04): a panic unwinds past
  it [kt-exit-finally].
* [throw] [rs-throw-controlflow] A fn declaring `[Throw<M>]` returns
  `std::ops::ControlFlow<M, T>` (`T` is `()` for a `None` fn) — the message
  type *is* `ControlFlow`'s `Break` payload, so the propagation falls out of
  the design rather than being imposed on it. `Throw` is filtered out of the
  effect *parameters* (it is a return shape, not a capability), and the
  effect declaration itself emits no trait: it has no handlers to implement.
  * `throw(m)` is `return std::ops::ControlFlow::Break(m);` — no handler, no
    dispatch, no allocation. `return v` becomes
    `return std::ops::ControlFlow::Continue(v);`, and a `None`-returning fn
    ends with `return std::ops::ControlFlow::Continue(());`.
  * A call that may throw is unwrapped by an explicit `match`, always:
    `(match crate::parse_port(config) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => return std::ops::ControlFlow::Break(__m) })`.
    That is an *expression*, so it works in argument position with no
    hoisting, and the same shape serves inside a `try` (where the break
    arm transfers to the label) and where the message must be wrapped into
    a union arm.
  * Closures do not throw: a lambda body has no `Throw` to propagate.
* [try] [rs-try-label] `try { ... }` is a **labelled block**
  (`('try_N: { ... })`), not a closure. A closure would have to capture the
  fn's effect parameters and any local the body mutates — an exclusivity
  trap; a labelled block captures nothing.
  * Every may-throw call in the body takes the `match` form above with the
    break arm leaving the label with the outcome's thrown arm
    (`break 'try_1 crate::unions::Union2::U2(__m)`, wrapped further when the
    thrown arm is itself a union:
    `break 'try_3 crate::unions::Union2::U2(crate::unions::Union2::U1(__m))`).
    The body's tail is wrapped into the `Ok` arm (arm 0,
    `crate::unions::Union2::U1(…)`); a body that always leaves still needs a
    value for the block, which the checker made `Ok None` [try].
  * Nesting needs no token: the label decides where a `break` lands
    (`'try_3` around `'try_4`), so an inner delimiter cannot swallow an
    outer throw [try-innermost].

## Qualifiers

* [is-qualifies] Each predicate qualifier's `qualifies` fn emits as a
  top-level `pub fn Q__Subject_qualifies(...) -> bool` [rs-fn-mangling]; a
  predicate `is` check becomes a call (`if crate::Positive__Int_qualifies(n)`,
  multiple qualifiers `&&`-chain). Parameters follow the ordinary modes
  [rs-borrows] (`person: &crate::Person`, `mut int: i32`).
* [qual-field-override] Field-override accesses read the value out of
  the declared representation, bound once where the claim applies:
  `let mut __claimed_1 = person.surname.as_ref().unwrap();` for a `T?` field
  claimed `T` (the cast-and-assert of [qual-field-override]; a wrong
  override panics on `unwrap`).
* [rs-fn-mangling] Rust has no overloading: every overload a module emits
  gets its own name by the shared rule [fn-emit-name] (`full_name__Person`,
  `full_name__SurnamePerson`, `next__Countdown`), at its declaration and at
  every call. Kotlin follows the same rule for a different reason — it *has*
  overloading, and would resolve by its own lattice [kt-fn-mangling].
* [implicit-param] [implicit-resolve] An implicit parameter emits as an
  ordinary trailing parameter, rendered like any fn-typed one:
  `cmp: &mut dyn FnMut(&T, &T) -> i32` [fn-contract]. The call site passes
  what resolution found — a resolved default as an adapter closure with
  typed parameters (`&mut |__a0: i32, __a1: i32| crate::plus(__a0, __a1)`,
  since a fn item is not a closure), an intrinsic default as its lowering
  inside the adapter [implicit-intrinsic], a forwarded one as a reborrow
  (`&mut *cmp`), an override as the written value (adapted the same way
  when it names a fn).
  * A `params` group emits nothing [implicit-group]: it never was a value, so
    there is no struct and nothing boxed; its members are trailing
    parameters like any implicit.
  * A **kept-`Mut`** parameter of an implicit position is rendered `&mut T`,
    and the adapter reborrows it into the callee
    (`&mut |__a0: &mut Vec<i32>, __a1: i32| crate::core_list::add_platform(&mut *__a0, __a1)`).
    A callee wanting `&T` takes the same value (`&mut T` coerces); one
    wanting it owned clones.
  * Every fn-typed parameter is `&mut dyn FnMut(..)` — **`dyn`,
    uniformly**: an effect member's fn parameters land in a trait used as
    `dyn` [rs-effects], where `impl Trait` in argument position would cost
    object safety, and forwarding has to compose in every direction (a
    member forwarding to a plain fn would otherwise hand a `dyn` value to an
    `impl` parameter). The cost is an indirect call, which every effect
    member call already pays.
  * A member's trait method, every handler's implementation of it, the
    handle's forwarding method and the generated host skeleton all render
    through the same member-parameter code, so they cannot disagree.
  * [effect-args-hoisted] An argument that calls a fn value or implicit the
    *same* call goes through is evaluated into an `__argN` first, or the two
    `&mut` borrows of the closure overlap (`E0499`):
    `return { let __arg1 = f(x); f(__arg1) };`, and a recursive call
    forwarding its own implicit
    (`{ let __arg1 = crate::sum(&vec![1i32], &mut *add); add({ … }, __arg1) }`).
    Only that argument is hoisted, so it is evaluated ahead of the arguments
    written before it: an open divergence from left-to-right order when
    those have effects (`f(noisy(1), f(noisy(2), x))` prints `noisy 2`
    first here and `noisy 1` first on Kotlin).
* [effect-handler-generics] A generic handler is constructed **at** a type:
  the `use` binds the instance with its instantiation written,
  `let mut __use_1: crate::Drop<i32> = crate::Drop::new();`, from the type
  arguments the checker resolved for the site — written even where rustc
  could infer them, since the case that needs them most (a handler with no
  constructor argument) leaves nothing to infer from. The handler's type
  parameters are bounded `T: Clone + Send + Sync + 'static`, since the
  instance sits in an `Arc` [rs-handle].
  * A type parameter **no field mentions** gets a
    `__phantom_T: std::marker::PhantomData<T>` field, initialized in `new`.
    A handler is a behaviour, and a generic one need hold nothing; Rust
    insists every parameter be used (`E0392`).
* [rs-fn-param-convention] A lambda passed into a fn-typed parameter binds
  its parameters the way the **callee's declared fn type** renders them,
  not the way the lambda's own annotation would: the callee fixes the
  calling convention, and its declaration is the only thing both sides can
  agree on. `f: (T) -> U` declares `FnMut(&T)` — a type variable is never
  known to be `Copy` — so even an annotated `(n: Int) -> …` lambda binds by
  reference. The lambda is emitted with **untyped** parameters
  (`&mut |mut n| -> i32 { i32::wrapping_mul(*n, 3i32) }`) and rustc takes
  their types from the `&mut dyn FnMut` it coerces to; a Copy scalar read
  out of one is dereferenced where a value is needed (`*n`).
  * **An implicit parameter's position follows the same rule.** A kept
    non-`Mut` position whose type is not a Copy scalar renders `&T`:
    `?Ordered<T>` is `&mut dyn FnMut(&T, &T) -> i32`, the body's `cmp(a, b)`
    passes the references, and the adapter that fills the position bridges
    to the resolved fn's own mode (cloning out of the borrow where that fn
    owns its parameter, which is free for a scalar):
    `crate::bigger::<String>(a, b, &mut |__a0: &String, __a1: &String| …)`.
    The same bridging applies to a value written at the call site
    [implicit-override]: a named fn gets the adapter, a lambda binds its
    parameters under the position's convention.
* [rs-cmp-groups] [cmp-groups] The canonical comparison/equality/hashing
  implementations are the host's own operations. `cmp` is
  `(Ord::cmp(&(a), &(b)) as i32)` — `Ordering` is a fieldless `#[repr(i8)]`
  enum whose discriminants *are* the sign convention Salvo's `cmp` answers,
  so the cast is the whole lowering — written as a path call so it works
  whether the argument arrives owned or borrowed. An operator on basic
  types never reaches these: the IR writes it as an `Op` [ir-op], rendered
  `(a < b)`, `(n == 0i32)`. `eq` is `==` (`((n) == (0i32))`) where a call is
  written or passed as a value. A `Str` is compared and
  hashed as `str` (`(&a[..] == &b[..])`), which is byte-wise UTF-8 and
  therefore code-point order [kt-ordered] — no runtime helper needed on
  this side.
  * [obligation-by] A member **stamped** by a `by` clause is an ordinary Rust fn
    whose body is the unrolled Salvo: `pub fn eq__Point_Point(a: &crate::Point,
    b: &crate::Point) -> bool` compares `a.x` with `b.x`, then `a.y` with `b.y`.
    Calls, adapter closures and `cmp = cmp@Point` values all reach it as a
    named fn, so nothing in this backend learns the member was not written by
    hand — and it takes part in overload mangling like any other body-bearing
    fn. The identities of a `List` or a tuple [col-hashed-ordered] are Salvo
    (`core.compare`), not lowerings; `mix_hash` is Salvo too.
  * [cmp-hash-values] `hash` is a block expression holding its own
    `std::hash::DefaultHasher`:
    `{ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(v), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }`
    (`&s[..]` for a `Str`). One hasher per call, so a `hash` nested
    inside another one is still well-defined, and the value is this host's —
    Kotlin's `hashCode()` answers something else by design.

## Effects [rs-effects]

* [effect-decl] An effect emits two traits and a handle struct
  [rs-handle]: `__Stateless_E` (`&self` members, `Send + Sync`) and
  `__Stateful_E` (`&mut self` members, `Send`), which handlers implement, and
  `E`, which everything else receives. The throw effect is the exception:
  it emits nothing, since it has no handlers [rs-throw-controlflow].
* [effect-args-hoisted] Two effectful calls in one expression need no hoist:
  a handle parameter is a shared `&E`, so both borrow it at once
  (`crate::roll(&__handle_4), crate::roll(&__handle_4)`). The hoist that
  remains is the fn-value one in [implicit-param].
* [effect-handler] Handlers emit as `pub struct H<..> { ctor-params,
  state, __dep0, … }` + `impl H { pub fn new(ctor-params, deps) -> Self }`
  (state fields initialized from their declared defaults) +
  `impl crate::__Stateless_E for H` or `impl crate::__Stateful_E for H`.
  Handler member bodies access ctor params, state and dependencies through
  `self.` (`self.t = i32::wrapping_add(self.t, 1i32);`,
  `self.__dep0.log(…)`). An `intrinsic handler`'s member bodies come from
  `intrinsics::handler_member` instead, same shape. A *dependency* is a
  `__depN: E` handle field and a trailing `new` parameter — see
  [rs-handle].
* [kt-effect-params]-equivalent: a fn's effects become leading parameters
  `name: &E<...>` [rs-handle], each **named for the effect** (`console`,
  `logger`; a second instance of one effect `random__1`):
  `pub fn draw(random: &crate::Random<i32>, random__1: &crate::Random<String>) -> String`.
  Effect member calls dispatch through the parameter
  (`random.next_random()`, `logger.log(step)`), and callee dependencies
  thread as arguments — the parameter as it is, `&__handle_N` for a handle
  `use`d in the current scope
  (`crate::draw(&__handle_4, &__handle_6, &__handle_2);`).
  * [deduce-syntax] **Effect member parameters follow the member's own
    written deduction clause**: consumed (`=> !s`) is by value, kept `Mut`
    is `&mut T`, kept plain is `&T`, Copy scalars and variadics by value
    (`fn log(&self, m: &String);`,
    `fn report(&self, what: String, done: crate::scheduler::SalvoReply);`).
    A member has no body to infer from, so [decl-explicit] makes the clause
    mention every non-Copy parameter — the clause *is* the contract. The
    trait methods, every handler's implementation, the handle's forwarding
    method and the argument rendering at call sites all read the IR's mode
    for that parameter, because a disagreement between any two of them is a
    rustc type error — and it is decided on the *effect's* parameter type,
    so a `T` position stays `&T` in a handler of `Store<Int>`. Member fns
    with their own generic parameters are a codegen error (`dyn` traits
    cannot have generic methods) [backend-never-wrong].
  * [effect-member-overload] **An overloaded member name is suffixed** by its
    parameter types [fn-emit-name] (`close__InStream`, `close__OutStream`) —
    Rust cannot overload a trait method at all. The name comes from
    `salvo_core::effect_member_name`, so the traits, every handler impl, the
    handle's forwarding method, the host skeletons and the call sites cannot
    disagree, and the Kotlin backend picks the same names. A call site emits
    the overload the checker resolved, as the IR records it.
* [effect-use] `use Handler(...)` binds the instance and then its handle:

  ```rust
  let mut __use_3: crate::TickingClock = crate::TickingClock::new();
  let __handle_4 = crate::Clock::locked(__use_3);
  ```

  (`shared` for a stateless handler), and the handle serves the effect for
  the rest of the scope [effect-scope]; ctor arguments are values (a `use`
  argument is a move, [deduce-infer]), and a generic handler's instantiation
  is the annotation [effect-handler-generics]. See [rs-handle].
* Which handle a call threads is the IR's: each effectful call names the
  effect instance it uses, already resolved to the parameter, the `use`
  local or the module-level `use` [rs-mod-use] in scope.

### Every binding is a handle [rs-handle]

One shape for every effect, keyed on statefulness and named after the effect
(user decisions 2026-09-28, ROADMAP §2b; COMPLETED.md's "One shape for
effects" keeps the history).

* **An effect emits two traits and a handle.** The traits are what handlers
  implement, and their names are the mangled ones; the handle **carries the
  effect's name** — it is what a reader of the output sees on every fn
  parameter, `use` binding and dependency field:

  ```rust
  pub trait __Stateless_Logger: Send + Sync { fn log(&self, m: &String); }
  pub trait __Stateful_Logger: Send          { fn log(&mut self, m: &String); }

  pub struct Logger { inner: __Inner_Logger }
  pub enum __Inner_Logger {
      Shared(std::sync::Arc<dyn __Stateless_Logger>),
      Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Logger>>),
  }
  impl Clone for Logger { … }                           // an `Arc` bump either way
  impl Logger {
      pub fn shared<__H: __Stateless_Logger + 'static>(inner: __H) -> Self { … }
      pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Logger>) -> Self { … }
      pub fn locked<__H: __Stateful_Logger + 'static>(inner: __H) -> Self { … }
      pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Logger>>) -> Self { … }
      pub fn log(&self, m: &String) {
          match &self.inner {
              __Inner_Logger::Shared(h) => h.log(m),
              __Inner_Logger::Locked(h) => h.lock().unwrap().log(m),
          }
      }
  }
  ```

  The handle's members are **inherent `&self` methods** dispatching on the
  arm, so a fn declaring `[E]` takes `e: &E` — a shared borrow, which also
  means two handles of one instance in one call never conflict. Generic
  exactly as the effect is (`pub struct Random<T: 'static>`); a generic
  instance's type arguments come from the binding's annotation
  [effect-handler-generics]. Emitted for **every** effect, actor effects
  included, used or not. Members with their own generics are skipped as the
  traits skip them. A mutable-lending member is dispatched through both arms
  [rs-loc]: it lends from a *parameter*, never from handler state, so the
  borrow outlives the guard legitimately.
* **Statefulness decides the arm** — `salvo_core::handler_is_stateful`, the
  one predicate both backends and the deadlock graph read, off the
  declaration: a `state` field; a member that mints a `replyto` (the parked
  table is state the mint writes); a fn-typed constructor parameter (a
  stored callback, and a lambda may mutate what it captured); a platform
  handler without `threadsafe`. An `intrinsic` handler is trusted stateless.
  The generated actor fields (`__addr`, `__parked`) do not count: the actor
  body owns the instance and writes them itself, so an actor handler with no
  state fields is `Shared` like any other. A **stateless** handler
  implements `__Stateless_E` with `&self` members and is shared as
  `Arc<dyn …>` with no lock; a **stateful** one implements `__Stateful_E`
  with `&mut self` and sits behind `Arc<Mutex<_>>`. Stateless by
  construction: the send stub `__Stub_E` (so a send through a `use addr`
  binding blocks only on the mailbox, never on a lock another sender holds
  while blocked on that mailbox), the mixed handler's façade `__Fac_H` (so a
  caller waiting on the servant holds nothing), intrinsic handlers,
  `threadsafe` hosts. A generic handler's parameters carry
  `Clone + Send + Sync + 'static`, since the instance sits in an `Arc`;
  every Salvo type satisfies them.
* **A `use` is the handle**: the instance is bound, then wrapped —
  `let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();`
  `let __handle_2 = crate::core_console::Console::shared(__use_1);` — and the
  handle is what everything downstream receives. A multi-face handler is one
  `Arc` (of a `Mutex` when stateful) and one handle per face:

  ```rust
  let __use_3 = std::sync::Arc::new(std::sync::Mutex::new(crate::fs_mem::MemFs::new()));
  let __handle_4 = crate::fs::Fs::share_locked(__use_3.clone());
  let __handle_5 = crate::stream::Streams::share_locked(__use_3.clone());
  ```

  so a **stateful multi-face handler is one lock behind several effect
  types**. A handler's `init` runs inside its `new`
  (`let mut __s = Self { … }; __s.init(); __s`), before the handle wraps it;
  an actor's is its first message instead [rs-actor]. A `use addr` of a
  plain effect binds the value (an `Addr<E>` *is* `E`); of an actor effect,
  `crate::E::shared(crate::__Stub_E::new(addr))`.
* **A fn declaring `[A, B]`** is `fn f(a: &A, b: &B, …)`, one parameter per
  effect in the order the IR lists the fn's effects — inherited effects
  (from fn-typed parameters, [fn-effects]) first, then the written list — and
  a call passes `&__handle_N` for a local handle, `&self.__depN` for a
  handler's dependency, the parameter itself for a parameter
  (`pub fn interception(logger: &crate::Logger, clock: &crate::Clock)`,
  called `crate::interception(&__handle_6, &__handle_4);`).
* **A dependent handler** ([effect-handler-deps]) holds one `__depN: E<…>`
  field per declared dependency, in declaration order and whatever the
  dependency's kind — plain, actor effect (a handle over the send stub), or
  generic instance (`Store<i64>`, or `<T>` for a generic handler) — taken as
  trailing `new` parameters:

  ```rust
  pub struct Stamped { __dep0: crate::Logger, __dep1: crate::Clock }
  impl crate::__Stateless_Logger for Stamped {
      fn log(&self, m: &String) {
          self.__dep0.log(&format!("[t={}] {}", self.__dep1.now(), m));
      }
  }
  ```

  Members reach them through the handle's `&self` methods, so a
  **stateless** dependent handler stays stateless. A `use` clones the
  scope's handles into `new` (`crate::Stamped::new(logger.clone(), clock.clone())`)
  whether the effect arrived by a `use` in the same function or through the
  enclosing *signature* ([spawn-inherit]: the parameter is the handle); a
  `with` item is a private instance behind its own handle
  (`crate::Stamped::new(__handle_6.clone(), crate::Clock::shared(crate::FixedClock::new()))`,
  `locked` when stateful). **Interception** ([effect-intercept]) is the
  same shape: the previous handle is cloned into the interceptor before the
  new handle is bound
  (`let mut __use_1: crate::Stamped = crate::Stamped::new(logger.clone(), clock.clone());`
  `let __handle_2 = crate::Logger::shared(__use_1);`), which is "binds
  strictly outward" in emission.
* **A spawn** hands the actor body the same struct: `__Actor_H { handler: H }`
  with the handler's fields the handles, `__dispatch_H_E` calling
  `crate::__Stateless_E::member(&mut *__handler, …)` (or
  `crate::__Stateful_E::…`, by the handler's statefulness) for every handler,
  dependent or not; the clause's items and the inherited dependencies are
  trailing `new` arguments exactly as at a `use`
  (`crate::Reporting::new(__handle_6.clone())`). A **mixed spawn** answers
  `crate::E::shared(crate::__Fac_H { __addr: __a, … })`: the façade is
  stateless, so its wait on the servant holds no lock [rs-mixed].
* **A fn value with effects** is `&mut dyn FnMut(&A, …, args)`: a lambda
  takes each handle as a leading parameter, and the adapter for a named fn
  forwards it (`|__e0: &crate::Logger, __a0: &String| crate::shout(__e0, __a0)`),
  ignoring what the declaration does not need [fn-effects]. **A task body**
  takes its inherited effects as *owned* handles — cloned into a local before
  the `move` closure (`let __e0 = console.clone();` …
  `std::boxed::Box::new(move |__v| crate::report(&__e0, …))`) — and clones
  them into nested mints.
* **`threadsafe platform handler`** ([threadsafe-platform]) is the declared
  statefulness of a host the compiler cannot see: a `threadsafe` host
  implements `EPlatformSync` with `&self` receivers and the `use` binds
  `crate::E::shared(__use_N)` — rustc checks the half of the contract it
  can, since a field that is not `Sync` does not compile — and an undeclared
  host implements `EPlatform` with `&mut self` behind `crate::E::locked(…)`
  [rs-platform-handler]. `std/platform/net.rs` (`HostTcpTransport`) is
  written in the first form.
* **Fn-typed constructor parameters** ([rs-fn-field]): a written fn-typed
  constructor parameter (`handler Derived(step: (n: Int) -> Int)`) is stored
  boxed — field `step: std::boxed::Box<dyn Fn(i32) -> i32 + Send + Sync>`,
  arriving as `step: impl Fn(i32) -> i32 + Send + Sync + 'static` and stored
  `std::boxed::Box::new(step)`, called `(self.step)(n)` — and makes the
  handler stateful. A Salvo lambda captures by value, so the bounds hold for
  every value the program can build, and the checker's "holds a function
  value" unsendability arm does not apply to handler state; it still applies
  to message payloads and task captures.
* **The handle is `Clone`, not `Copy`** — unlike the `usize` addr — and the
  checker treats every `Addr` as freely reusable, so a value read of a
  plain-effect addr **clones**. Both arms are `Send + Sync`, which is the
  sendability the checker promises at a spawn [actor-sendable].
* **Rust's `Mutex` is not reentrant, and that is unobservable**: a handler's
  bindings are fixed at construction and its dependencies bind strictly
  outward/earlier, so no path routes back into its own handle; sibling calls
  inside a handler are direct self calls under the one acquisition. A
  poisoned lock (`unwrap`) surfaces as a panic only after another member
  already panicked, which is the fault boundary's business.
* **`Addr<E>` for a plain effect lowers to `E`** [monitor-handler] (the type
  renderer branches on the effect's declared kind); an actor effect's addr stays `usize`. A
  generic plain effect's addr carries the instantiation (`Addr<Random<Int>>`
  is `crate::Random<i32>`).

## Functions and calls

* [fn-dot] Dot-notation calls resolve to a declared fn or effect
  member and normalize to `f(base, args)`. There is no method-call
  fallback: an unresolved name here is a *codegen error* naming an internal
  inconsistency, since the checker already rejects undeclared dot-calls
  ([call-resolve]).
* [fn-variadic] Non-spread trailing arguments collect into `vec![...]`
  (`crate::total(vec![4i32, 5i32])`); with a spread the vector is built in a
  block, each spread extending it with clones of its elements:
  `crate::total({ let mut __v = Vec::new(); __v.push(1i32); __v.extend(rest.iter().cloned()); __v })`.
* [fn-lambda] Lambdas emit as closures with a written return type
  (`&mut |mut a, mut b| -> i32 { … }`); fn-typed parameters emit as
  `&mut dyn FnMut(A, ..) -> R` [fn-contract]. Lambda parameters take their
  mode from the position [rs-fn-param-convention]. A `return` inside a
  block-bodied lambda is the closure's own `return` (`&mut |mut n| -> i32 { if (n > 0i32) { return 1i32; }; return 2i32; }`).
* [intrinsic-fn] Every std lowering lives in this crate's `intrinsics.rs`
  (`fn_call`), keyed by the checker-resolved declaration (name + first
  parameter's base type name, so `size(Str)` / `size(List<T>)` /
  `size(T[])` are distinct entries). It is the one place the emitter reads
  the AST [rs-ir]. `copy` [rs-copy] and `discard` [linear-discard] are the
  exceptions: they dispatch on the argument's type, so the emitter lowers
  them itself. An intrinsic with no entry is a codegen error naming it.
  Most of std's surface is not intrinsic at all but **platform fns**,
  called through their wrappers (`crate::core_list::add_platform::<i32>(&mut xs, 3i32)`,
  [platform-fn]).
  * The arguments are rendered by the IR's pass mode for each parameter, as
    for any call [rs-read-mode]: a lent one as the place (or reference) the
    lowering borrows, a consumed one as a value, a variadic tail as values,
    since it lands inside `vec![..]`.
  * Rust does not spell type arguments out the way Kotlin must: `vec![]`
    stays `vec![]`, since [call-type-args] guarantees the element type is
    either written or annotated, and both reach rustc through the rendered
    `let` annotation or parameter type. A lowering *is* handed the call's
    resolved type arguments for the rare construct that needs them spelled
    out.
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
  `crate::wire::salvo_encode(&v)`, `decode<T>(b)` to
  `crate::wire::salvo_decode::<T>(&b)` with `T` from the call's checked type
  argument.
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
  mounts the other. Generated code (all under `crate::scheduler::`):
  `salvo_spawn(pool, bound, body, crate::__DECODE_H)`,
  `salvo_send_wire(addr, crate::__Msg_E::Member(args), crate::__PROTO_E)` for
  every send on a protocol with a wire form — the message unboxed, the runtime
  boxes it — and `salvo_send(addr, std::boxed::Box::new(msg))` otherwise (a
  private message), `salvo_reply_wire::<T>(tok, v)` for a `send(reply, v)`
  whose payload has one (`(tok).send(std::boxed::Box::new(v))` otherwise),
  `salvo_waiter_decoder(__wid, dec)` after every `salvo_waiter()`,
  `salvo_mint_task(pool, body, dec)`; the actor body implements
  `decode_reply`, and a free
  `pub const __DECODE_H: Option<crate::scheduler::MsgDecoder>` sits beside it
  (free, since a generic dependent body could not name an associated const;
  `None` when no protocol of the handler has a wire form).
  Frames staged under the lock go out through a thread-local outbox flushed
  after every release (`flush_out`), including in `run_job` for the credit a
  dequeue grants.
* [rs-wire] [effect-generic-decl] [actor-group] The emitter builds against
  the program with erased effect generics
  (`salvo_core::erase_effect_generics`): the type of an erased name renders
  without arguments, and a call to an erased fn takes no turbofish.
  `#![allow(…)]` carries `non_upper_case_globals` for the free
  `__DECODE_*`/`__PROTO_*` constants. Runtime: `salvo_pending(addr)` (queue depth
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
  fills an implicit `?protocol: () -> Protocol<E>` (the adapter is
  `&mut || crate::…::Protocol { name: "E".to_string(), hash: crate::__PROTO_E.to_string() }`,
  a generic caller forwards `&mut *protocol`) [implicit-intrinsic] —
  `watch_control`, `send_control`,
  `control_frame`, `node_left`, `local_protocols`, `set_peer_protocols`,
  `peer_protocol`, `pending`, `node_of`, and `eq(Addr, Addr)` as `==` on the handle.
  `actor_group<E>` is ordinary Salvo since 2026-09-28 [actor-group] (two
  overloads, `actor_group__Addr` and `actor_group__Str_Addr` in the output).
* [rs-wire] [route-stub] Runtime: the views live in the routing service
  (`runtime.routing`'s `view_set`, `view_members` sorted by `(node, actor)`,
  `view_version`, `view_refresh`, `view_wait`); `salvo_key_hash` (FNV-1a 64
  over the wire bytes) stays host-side. Intrinsics: `view_set`,
  `view_members`, `view_version`, `view_refresh`, `view_wait`, `key_hash`
  (over `salvo_encode`). `use route_any(g, c)` emits the construction of
  the checker's `route_stubs[site]` handler exactly as a written `use
  __Route_E(g, c)` would. An erased effect's types render without arguments
  everywhere, dependencies and `use` bindings included — `RouteSelector`,
  `dyn __Stateful_RouteSelector`, `Sharded::new()`. A `while` is a `loop`
  that tests first (`loop { if !(cond) { break; }; … }`), `while true`
  included.
* [rs-actor] [handler-init] `init` is an inherent `fn init(&mut self)`. On an
  actor handler it is one more private member, `__Priv_H::Init`, dispatched
  by `__dispatch_priv_H` (`__handler.init();` in the `Init` arm): a spawn
  emits `crate::scheduler::salvo_send(__a, std::boxed::Box::new(crate::…::__Priv_H::Init));`
  right after `salvo_spawn` and before the addr is answered. On any other
  handler `new` runs it: `let mut __s = Self { … }; __s.init(); __s` — a
  dependent handler's dependencies are its own fields, so `init` is a plain
  method like every other member [rs-handle].
  `self@Face` lowers to `self.__addr.expect("an actor's own addr")`; the
  `watch_control` intrinsic builds a `__Priv_{current handler}::Control`
  variant
  (`crate::scheduler::salvo_watch_control((channel).clone(), (sink).clone(), |__n, __d| std::boxed::Box::new(…::__Priv_H::Control(… { id: __n as i64 }, __d)))`),
  since a control frame's payload arrives as the handler's own private member.
* [rs-actor] [actor-private-send] A private send member is an **inherent
  method** on the handler struct (`impl H { fn k(…) }`) with owned
  parameters (its written clause is all-consumed). `__Priv_H` is the
  handler-keyed enum of its private messages; `__Actor_H::handle` tries it
  after the faces' enums, `__dispatch_priv_H` calls the method, `resume`
  rebuilds a `__Priv_H` for a continuation variant of `__Cont_H` that names
  a private member, and `decode_reply` covers it by its answer type.
  `k@self(…)` evaluates the payload once and sends it or runs it inline,
  by whether `__addr` is set:
  `{ let __s0 = 1i32; match self.__addr { Some(__a) => crate::scheduler::salvo_send(__a, std::boxed::Box::new(crate::__Priv_Reporting::Twice(__s0))), None => self.twice(__s0) } };`.
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
  | `Map<K, V>`, `Set<T>` | `crate::core_map::Map` / `crate::core_set::Set` (std's host types, re-exported) — insertion-ordered; build the canonical one with `crate::platform_core_map::canonical_map(entries)` / `crate::platform_core_set::canonical_set(elems)` [rs-collections] [platform-check] |
  | `SortedSet<T>`, `SortedMap<K, V>` (canonical ordering) | `crate::core_sorted::SortedSet` / `SortedMap`; build one with `canonical_sorted_set` / `canonical_sorted_map` (Rust's `Ord`, which is Salvo's order for every type that may cross) [platform-check] |
  | `Deque<T>`, `Bytes` | `std::collections::VecDeque<T>`, `Vec<u8>` (std's host aliases) [rs-deque] [bytes-type] |
  | a union, to build | its factories: `FsError::not_found(…)`, `ReadToStr::ok(…)` [platform-factory] |
  | `T?` | `Option<T>` [rs-option] |
  | a union of *n* ≥ 2 runtime arms | `crate::unions::UnionN<A, …>` with variants `U1`…`Un` in runtime-arm order [union-arm-identity]; `Some(…)` around it when it has a `None` arm |
  | `struct S { f: T }` | `pub struct S { pub f: T }`, built with a literal of every field; a dot-name `A.B` is `AB`; a field that is a Rust keyword is `r#f` |
  | `Checked<T>` | `crate::core_checked::Checked { value: T }` |
  | `Reply<T>` parameter | `crate::scheduler::SalvoReply`; `.hosted()` answers the `SalvoHostReply` whose `send(v)` may run on any thread, exactly once [platform-reply] |
  | `InStream` / `OutStream` | `crate::stream::InStream { handle: i64 }`; the table is Salvo (`runtime.streams`); host code reaches it through `crate::hoststreams::salvo_stream_register_in/out` and `salvo_stream_take_in` (an `Arc<Mutex<SalvoIn>>` reader) [stream-table] |
  | `platform handler H(p: T) of E` | `pub struct H` with `pub fn new(p: T) -> Self`, implementing `crate::<module of E>::EPlatformSync` (`&self`) when `threadsafe`, `EPlatform` (`&mut self`) otherwise [rs-platform-handler] |
  | a member parameter | kept non-`Copy`: `&T`; kept `Mut`: `&mut T`; consumed, or `Copy`: `T` [rs-borrows] |
  | a fn-typed parameter | lent: `&mut dyn FnMut(…)` (the wrapper hands it on as `&mut name`, which also fits a host's `&mut impl FnMut`); `once`: `impl FnOnce(…)`; **kept** (`=> !hook`) outside the runtime: the pointer `fn(…) -> R`, the call site's capture-free adapter coercing to it [platform-fn-value]; kept in the runtime: `Box<dyn FnOnce/FnMut + Send + 'static>` [runtime-kept-fn] |

* [rs-mod-use] [mod-use] A module-level `use` emits accessor fns over
  `std::sync::OnceLock` statics: `pub fn __module_useN() -> &'static T` for
  the instance (`T` is the `Arc`, or `Arc<Mutex<_>>` when stateful), and one
  `pub fn __module_useN_M() -> &'static crate::m::E` per face, whose
  initializer wraps the instance in the effect's handle:

  ```rust
  pub fn __module_use0_0() -> &'static crate::runtime::RuntimeHost {
      static CELL: std::sync::OnceLock<crate::runtime::RuntimeHost> = std::sync::OnceLock::new();
      CELL.get_or_init(|| crate::runtime::RuntimeHost::share_shared(crate::runtime::__module_use0().clone()))
  }
  ```

  A call that uses the effect threads the accessor where a declared effect
  threads its parameter (`crate::runtime::__module_use1_0().is_virtual()`).
* [rs-host-fields] A struct whose fields reach a host value — any platform
  type, or a `Reply` — derives what those support: no `Debug`/`PartialEq`
  (a hand-written `Debug` prints such a field as `<fn>`, as for a fn field),
  and no `Clone` at all when the value is linear (a linear platform type, a
  reply token). The test walks type arguments and named structs' fields.
* [rs-linear-move] A binding taking a linear payload out of one **arm of a
  union** held in a local (`while r is Full f`) moves it with a `match`, as
  the plain-optional shape moves with `unwrap` (`let mut next_1 = next.unwrap();`);
  so does a narrowed linear arm handed on, and destructuring a narrowed
  linear struct (`let {a, b} = x`).
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
  `&mut dyn FnMut(…)`, a `once` one included
  (`fn once_apply(&self, f: &mut dyn FnMut() -> i32) -> i32;`, called with
  `&mut || -> i32 { … }`): effect traits are used as `dyn`, and an `impl`
  parameter would make the trait not object-safe. A top-level fn takes a
  lent one as `&mut dyn FnMut` too, and a `once` one as `impl FnOnce`.
* [rs-platform-never] [platform-never] `Never` is `()` everywhere in Rust
  output except a platform signature's result: a `platform fn`, its wrapper
  and skeleton, and an `EPlatform`/`EPlatformSync` member are `-> !`. The
  adapter's impl of the effect's own trait keeps `-> ()` and forwards the
  host's `!`, which coerces.
* [rs-platform-factory] [platform-factory] A named union gets `pub type FsError =
  Union7<…>;` (Rust otherwise spells union aliases out) and `impl FsError {
  pub fn not_found(value: NotFound) -> Self { crate::unions::Union7::U1(value)
  } … }` — an inherent impl on one instantiation of a crate-local type. A
  signature's union gets `pub struct ReadToStr;` with an `impl`. Two named
  unions over the same instantiation with a shared arm name collide in rustc
  rather than silently.
* [rs-platform-check] [platform-check] **Collections** (D10 C1, C2): Rust's
  types promise insertion order and the ordering, so a plan's `Shape` nodes
  are dropped (`BoundaryCheck::without_shapes`); the hosts' `canonical_*`
  builders build each under the canonical identity, the only one a host may
  return.
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
  [rs-handle]: `let mut __use_3: crate::__Platform_HostRawClock = crate::__Platform_HostRawClock::new(35i32);`
  `let __handle_4 = crate::RawClock::locked(__use_3);`. `M` is the module
  that *declared* the handler. Emitted only when the host file exists (or
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
    receivers and its `use` binds `E::shared(__use_N)` — no lock, and
    rustc refuses a host whose fields are not `Sync`; an undeclared host
    implements `EPlatform` with `&mut self` behind `E::locked(…)`, so a
    host that did not claim safety behaves identically on both backends and
    pays only the lock.
  * **The skeleton prints the contract** in both shapes: the threadsafe one
    opens with the signing comment and implements `<effect_path>::
    EPlatformSync` with `&self` receivers; the undeclared one says the
    compiler serializes the instance and implements `EPlatform` with
    `&mut self`. Regenerating after toggling the word changes the receivers.
* [rs-copy] `copy(x)` lowers to `(place).clone()` on the argument's place
  (`let mut t: String = (s).clone();`, `let mut q: crate::Person = (p).clone();`),
  whatever the binding's mode — every generated type derives or is `Clone`,
  and generic parameters carry a `Clone` bound; a narrowed name clones its
  narrowing binding; constructed values (call results, literals) pass
  through — they are already fresh, so `copy` is free on them.
* [struct-defaults] Rust has no default arguments: struct literals inline the
  declared default expressions for omitted fields at every literal site
  (`crate::P { name: String::from("a"), age: 3i32, nick: None }`), the
  struct named by its path [rs-default-path].
* [fn-contract] Fn-typed parameters emit `&mut dyn FnMut(…)` — the value is
  borrowed (closure double-use works; `FnMut` accepts handler-mutating
  closures), with argument types per the contract: kept non-Copy `&T`, kept
  `Mut` `&mut T`, moved or Copy by value. A call through a fn value renders
  its arguments per that contract (`f(x)`, `f(console, …)` with effects
  first); a lambda binds its parameters under it; a named fn passed by value
  wraps in an adapter closure with typed parameters bridging the contract's
  convention to the declaration's own modes
  (`&mut |__a0: &Vec<crate::Person>| crate::count(__a0)`,
  `&mut |__a0: &i32| crate::double(*__a0)`).
* [once-fn] `once` fn parameters emit `impl FnOnce(…)`
  (`pub fn once_it(f: impl FnOnce(i32) -> i32) -> i32`), and a lambda filling
  one is a `move` closure with typed parameters
  (`crate::once_it(move |mut a: i32| -> i32 { a })`). Calling the parameter
  is a plain call (the by-value `call_once` is implicit).
* [linear-discard] `discard(x)` lowers to `std::mem::drop(x);` on the moved
  value [intrinsic-fn]; linearity itself is purely static [linear-static] —
  no `#[must_use]`, no `Drop` impls are generated.
* [struct-spread] `P {...p, f: v}` binds the base once and builds the literal
  field by field, moving the fields not written out of it:

  ```rust
  {
      let mut __spread_7: crate::P = <base>;
      crate::P { name: __spread_7.name, age: 9i32, nick: __spread_7.nick }
  }
  ```

  The base is a value (a clone when the read does not consume it), so the
  copy is deep where Kotlin's `.copy()` is shallow on `Mut` fields — which is
  unobservable, because the checker consumes the spread base
  [deduce-consume]. Several spreads each bind a base (`__spread_3`,
  `__spread_4`), the later one supplying a field both have; a base no field
  is taken from is bound by reference (`let mut __spread_3 = &p;`).

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
  subject's base name disambiguates — `Filled__List_qualifies` — **always**
  since 2026-10-05 [fn-emit-name], so whether another module declares a
  qualifier of the name does not change this one's; a qualifier with a generic
  subject keeps `Q_qualifies`. The predicate call site resolves the
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
  `std::collections::VecDeque<T>` (std's host aliases them,
  `pub type Deque<T> = VecDeque<T>;`), and the surface is std's platform fns
  over it, called through their wrappers like any other
  (`crate::core_deque::size_platform::<i32>(&dq)`,
  `crate::core_deque::remove_first_platform::<i32>(&mut q)`;
  `deque_of(1, 2)` is `crate::core_deque::deque_of__T_TArray::<i32>(1i32, vec![2i32])`).
* [rs-collections] The keyed collections are **std's host types**
  (2026-10-06, ROADMAP §0j step 7): `Set<T>`/`Map<K, V>` in
  `std/platform/core/{set,map}.rs`, an insertion-ordered slot vector of
  `(digest, entry)` with tombstones and a `HashMap<i64, Vec<usize>>` from a
  Salvo digest to its slots, compacted when the tombstones outgrow the live
  entries; the sorted pair a sorted `Vec` searched with `binary_search_by`
  (`sorted.rs`). They render as ordinary platform types (`crate::core_set::Set`
  re-exported from the host file) and need no runtime module.
  * **Every operation takes the identity as trailing lent fn parameters**
    (`hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool`,
    `cmp: &mut dyn FnMut(&T, &T) -> i32`) [platform-slots]: nothing is stored,
    so the old marker types (`__Hash_…`, `HostHash`), the value-keyed stores
    and the owned-`Arc` convention for a kept capability
    ([rs-stored-implicit], deleted) are gone. A Given intrinsic identity
    renders as its lowering inside the adapter [implicit-intrinsic].
  * `PartialEq` is order-blind (each entry found under its cached digest, by
    the elements' own `==`), `Debug`/`Display` print `{a, b}` / `{k: v}`;
    `canonical_hash`/`canonical_set`/`canonical_map`/`canonical_sorted_set`
    build the canonical case for host code.
  * [linear-container] `replace(key, value) -> Option<V>` and
    `into_values() -> Vec<V>` (what `drain(map, each)` walks) are the
    obligation surface.
* [rs-mailbox] [actor-mailbox] **The mailbox bound is a generated field on the
  handler**, `__mailbox_capacity: i32`, initialised by `new` from the slot's
  expression — which is exactly where a state field's initialiser is computed,
  and the reason the slot's expressions are confined to constructor parameters:
  in `new`'s scope they are simply in scope.
  * A spawn therefore reads the bound **off the instance**, before it moves
    into the actor body:
    `({ let __h = crate::Reporting::new(__handle_6.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::core_actor::pool(1i32), __cap as usize, std::boxed::Box::new(crate::__Actor_Reporting::new(__h)), crate::__DECODE_Reporting); __a })`.
    The ordering is the point — the
    alternative (emitting the slot's expression at the spawn site) would need
    the constructor arguments in scope there, and would evaluate them twice.
* [rs-is-hoist] [is-bind-once] **A non-place `is` subject becomes a
  temporary**, read by both the test and the binding:
  `let mut __subject_13: Option<&String> = …;` before an `if`, and inside the
  `loop` for a `while`:

  ```rust
  loop {
      let mut __subject_1: Option<i32> = crate::core_list::remove_first_platform::<i32>(&mut *xs);
      if !(__subject_1.is_some()) {
          break;
      };
      let mut n = __subject_1.unwrap();
      …
  }
  ```

  The IR binds the subject once, so the test, the binding and any nested
  read all name the same temporary.
* [rs-state-take] [linear-state] **Taking a container out of handler state is
  `std::mem::take`.** A field behind `&mut self` cannot be moved out (E0507),
  and cloning it would duplicate every obligation inside — so a consuming
  read of a state field (the IR's consume mark on a `self` place) renders as
  `std::mem::take(&mut self.waiting)`
  (`crate::core_list::drain::<crate::scheduler::SalvoReply>(std::mem::take(&mut self.waiting), …)`).
  That is also the honest semantics: the field is empty until the member puts
  something back, which [linear-state] requires it to do before returning.
  `mem::take` needs `Default`, which `Vec` has — and a *bare* obligation in
  state (whose type need not) is refused by the checker, so the emitter never
  meets one.
* [rs-linear-move] [linear-container] **A narrowed linear value is moved, not
  cloned.** A value read of a narrowed optional is otherwise a clone through
  the borrow (`x.as_ref().unwrap()` … `.clone()`),
  which for `remove_first(waiting)`'s `Option<SalvoReply>` would duplicate a
  one-shot token — and `SalvoReply` is deliberately not `Clone`, so it would
  not even compile. Where the IR marks the read of a linear value a consume,
  the plain-optional shape renders as
  `let mut next_1 = next.unwrap();`, moving the payload out; the checker has
  consumed the variable, so nothing reads it again. A narrowed *union arm*
  moves out with a `match` [rs-linear-move].
  * **The `is`-binding site too**: `remove_at(pending, i) is Reply<Fired>
    token` ([time-manual]'s deadline queue) binds by moving out of the `Option` (`.unwrap()` on the subject's temporary).
  * `drain(list, each)` and a map's `drain` are std Salvo over
    `remove_first`/`into_values`, so they need nothing of their own here.
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
  * **It travels with the host stream table**: it is emitted wherever
    `hoststreams.rs` is, which ships wherever the scheduler or the wire does.
    `scheduler.rs` is a shim onto the runtime module's core (2026-10-03) and
    holds the `Addr`/`Reply` codecs, so `wire.rs` stands alone; the runtime
    tests compile it inside the files example's generated tree.
* [rs-time] [time-timer] **Deadlines are the runtime module's**, in Salvo
  (2026-10-02): the scheduler has no timer thread, and `HostRuntime.mono_nanos`
  reads `crate::hosttime::salvo_mono_nanos()`, the timeline `tick()` reports.
* [rs-default-path] **A struct literal's type is spelled so it resolves where
  it is written**: a field default is inlined at the outer literal, whose file
  need not import the type the default names — which every literal satisfies,
  since every struct is written through its path (`crate::m::S { … }`,
  [rs-imports]).
* [rs-fn-value-nested] **`f(f(x))` evaluates the inner call first**: a fn
  value is called through `&mut`, so an argument calling the same value
  borrows it twice (E0499); that argument goes into a local first —
  `{ let __arg1 = f(x); f(__arg1) }` — ahead of the arguments before it
  ([effect-args-hoisted] records the order this costs).
* **Generated code spells `std::boxed::Box`** (2026-10-04): a program's effect
  named `Box` made the generated `Box::new` resolve to its handle type.
* [rs-mailbox] A handler's `__mailbox_capacity` field is **`pub`**
  (`pub __mailbox_capacity: i32,`, initialised `__mailbox_capacity: 4i32`): the
  spawn site need not be in the same module, and std's own `DefaultTimer` is
  spawned from user code [backend-never-wrong].
* [rs-mixed] **The mixed lowering** [mixed-handler] (SH-1, built
  2026-09-19). A mixed handler splits into two generated types plus the
  servant's runtime parts:

  * **The handler struct is the servant alone**: state + ctor params + the
    actor fields (`__mailbox_capacity`, `__addr`, and `__parked` when a
    send member could be a continuation target); `send fn` members are
    **inherent methods** (`fn advance(&mut self, out: crate::scheduler::SalvoReply)`;
    no trait declares them), their parameter modes from their own written
    all-consumed clause, so payloads are owned exactly as the message enum
    carries them. Sync members are not emitted here at all.
  * **`__Priv_H` + `__Cont_H` + `__Actor_H`**: the servant's messages are the
    handler's private ones — `pub enum __Priv_CyclicRandom { Advance(crate::scheduler::SalvoReply) }`,
    one variant per `send fn` member — with the continuation enum and actor
    body beside them: `handle` downcasts `__Priv_H` into `__dispatch_priv_H`,
    which calls the inherent method (`__handler.advance(out);`),
    and `resume` removes the parked continuation and calls the member with
    the downcast answer as its trailing argument [defer-deduction].
  * **`__Fac_H`**: `#[derive(Clone)]`, `pub __addr: usize` plus the ctor
    params (owned), implementing each plain face's `__Stateless_E` with the
    sync member bodies — emitted with the ordinary handler-member code (ctor
    params resolve as self fields; state never resolves, the checker confined
    it). A façade send evaluates its payload and sends unconditionally:
    `{ let __s0 = got; crate::scheduler::salvo_send(self.__addr, std::boxed::Box::new(crate::__Priv_CyclicRandom::Advance(__s0))) };`.
  * **A servant send** [actor-self-send] — a bare sibling call or `k@self(…)`
    in a send member — is the ordinary self-send [rs-actor]:
    `{ let __s0 = out; match self.__addr { Some(__a) => crate::scheduler::salvo_send(__a, std::boxed::Box::new(crate::__Priv_Chain::Relay(__s0))), None => self.relay(__s0) } };`.
    A façade `k@self(…)` lowers exactly as the bare façade send does.
  * **The mixed spawn** evaluates ctor args once (`let __c0 = …;`), clones
    them into the handler, moves them into the façade, reads the mailbox
    bound off the instance, spawns `__Actor_H`, and answers the façade
    behind the effect's handle, stateless so lock-free [rs-handle]:
    `({ let __c0 = 12345i32; let __h = crate::CyclicRandom::new(__c0.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_current_pool(), __cap as usize, std::boxed::Box::new(crate::__Actor_CyclicRandom::new(__h)), None); crate::Random::shared(crate::__Fac_CyclicRandom { __addr: __a, seed: __c0 }) })`.

* [rs-actor] **Asynchronous effect handlers** lower to three generated
  pieces plus one shipped runtime module, `runtime/scheduler.rs`
  [rs-runtime-source] — emitted, and mounted as
  `#[path = "scheduler.rs"] pub mod scheduler;`, only into a program that
  spawns:
  * **The protocol's message enum**, `__Msg_E`, in the effect's module: one
    variant per `send fn`, owning its payload (an IR `Decl::Enum`, rendered as
    a `pub enum` [rs-actor-ir]). It is the *effect's*,
    not a handler's, because a sender holds an `Addr` and knows only the effect
    it serves — the same reason an actor and a locally `use`d handler are
    interchangeable [actor-types].
  * **The actor body**, `__Actor_H`, beside the handler: a struct owning the
    handler instance (`pub struct __Actor_Reporting { handler: Reporting }` —
    an actor's state *is* the handler's) implementing
    `crate::scheduler::SalvoActor`, whose `handle` writes `__addr`, downcasts
    the message enum
    (`let msg = *msg.downcast::<crate::__Msg_Reporter>().expect("message of this protocol");`)
    and dispatches it.
    * Member invocation lives in **one** place per protocol, a free function
      in the handler's module, `pub fn __dispatch_H_E(__handler: &mut crate::H, mut __msg: crate::__Msg_E)`
      (generated IR, not part of `__Actor_H`). One arm per variant
      unpacks the payload and calls the member through its trait:
      `if matches!(__msg, crate::__Msg_Reporter::Report(..)) { let crate::__Msg_Reporter::Report(what, done) = __msg else { unreachable!() }; crate::__Stateless_Reporter::report(&mut *__handler, what, done); }`.
      `handle` calls it as `crate::__dispatch_H_E(&mut self.handler, msg)` and
      `resume` rebuilds a message for the same function.
  * **The parked-continuation table, and the address, live on the handler.** A
    handler of an `actor effect` carries two generated fields, whichever way it
    is bound — a handler is compiled once:
    * `__addr: Option<usize>` — written by `handle`/`resume` from the
      activation's `SalvoCtx` before the member runs, and `None` when the
      instance was bound with `use` instead. That absence is the **self-send's
      discriminator** [actor-self-send].
    * `__parked: HashMap<u64, __Cont_H>` — slot → continuation, emitted when
      the protocol has any member that could be a target.

    They sit on the *handler* rather than on `__Actor_H` because the **mint**
    happens in a member body, which holds `self` on the handler and cannot see
    the actor struct; `resume` reaches them through `self.handler` (user
    decision 2026-09-15, D5-b). The alternative — a table in the runtime —
    would have changed `SalvoActor::resume`'s decided signature, and the two
    runtimes are the most exactly-mirrored code in the phase.
  * **[effect-handler-multi] A handler of several effects is one actor with one
    dispatch function per protocol.** One `impl crate::__Stateful_E for H` (or
    `__Stateless_E`) per face (a member that
    implements a same-named member of two faces appears in both impls — Rust
    cannot share a method between two traits, and the signatures are identical
    wherever that is legal, so the body is emitted twice rather than
    forwarded); one `__dispatch_H_<Effect>` function
    per face; and a
    `handle` that asks each protocol in turn —
    `match msg.downcast::<crate::__Msg_Timer>() { Ok(__m) => return …, Err(__m) => __m }`
    hands the box back on a miss, which is what makes the chain possible,
    with `__Priv_H` asked last when the handler has private members. `spawn`
    answers `(__a, __a)`: one scheduler index, one mailbox, one tuple element
    per face, so least authority costs nothing at run time.
    * A multi-face `use` is one `Arc<Mutex<H>>` and one handle per face
      [rs-handle], so a fn declaring `[A, B]` bound to one handler's two
      faces borrows two handles of one instance.
  * **The continuation enum**, `__Cont_H` (an IR enum, rendered as a `pub enum`), emitted beside the **handler** whose
    members it names — the handler, not the effect, because a mint is lexical
    ([effect-handler-multi]: with several faces a handler's members come from
    several protocols, and `replyto` targets the *handler's*). Its variants are
    named after the effect member each one resumes: one variant per send member
    with at least one parameter, carrying that member's parameters **minus the
    trailing one**. The last parameter is the answer itself [actor-replyto],
    which arrives with the reply rather than being stored — so the variant tells
    `resume` both *which* member to call and *what type* to downcast the answer
    to (`Report(String)` for `report(what: Str, done: Reply<Int>)`; a member
    whose only parameter is the answer gets a unit variant, `Advance`). A
    parameterless member gets no variant: there is no answer for a token to
    carry.
  * **`resume`** writes `__addr`, pops the slot
    (`let Some(__cont) = self.handler.__parked.remove(&slot) else { return; };`
    — a reply whose continuation is gone returns silently), matches the
    variant, downcasts `value` to the trailing parameter's type, and hands a
    rebuilt message to the dispatch function:
    `__Cont_Reporting::Report(what) => crate::__dispatch_Reporting_Reporter(&mut self.handler, crate::__Msg_Reporter::Report(what, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer")))`
    (a `__Priv_H` to `__dispatch_priv_H` for a private member).
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
    management) could sit next to it.
  * **The forms** (runtime calls under `crate::scheduler::`): `spawn H(args) on P` →
    `({ let __h = crate::H::new(args); let __cap = __h.__mailbox_capacity; let __a = salvo_spawn(P, __cap as usize, std::boxed::Box::new(crate::__Actor_H::new(__h)), crate::__DECODE_H); __a })`,
    whose value is the addr — the bound is read off the instance because it
    is the *handler's* [actor-mailbox], and a dependent handler's handles are
    trailing arguments of its own `new` [rs-handle];
    `addr.member(args)` →
    `salvo_send_wire(addr, crate::__Msg_E::Member(args), crate::__PROTO_E)`
    [rs-wire]; `waitfor out: Reply<T> { … }` → a block expression that mints
    a waiter, runs the block, then waits and downcasts to `T`:

    ```rust
    {
        let (mut done, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, …);
        crate::scheduler::salvo_send_wire(r, crate::__Msg_Reporter::Report(String::from("inherited"), done), crate::__PROTO_Reporter);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    }
    ```

    `send(r, v)` → `salvo_reply_wire::<T>(r, v)`, or
    `(r).send(std::boxed::Box::<T>::new(v))` for a payload with no wire form;
    `pool(n)` is std Salvo (`crate::core_actor::pool(1i32)`, over the runtime
    module); `thread()` → `salvo_thread()`; `watch` and `on_idle` are Salvo
    over `reply_token(r)` → `(r).take_local()` [runtime-handles].
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
    `{ let __c0 = …; crate::scheduler::salvo_mint_task(P, std::boxed::Box::new(move |__v| crate::k(__c0, …, *__v.downcast::<Payload>().expect("the awaited answer"))), <decoder>) }`,
    with `crate::scheduler::salvo_current_pool()` for an omitted `on`
    [task-pool-inherit]. No continuation enum, no slot, no
    `__parked` entry: the closure *is* the continuation, which is why a task
    needs no dispatcher. The captures are bound to `let`s **outside** the
    closure so they are the values as they were at the mint, and the closure is
    `move` so it owns them.
    * The `Box<dyn FnOnce(SalvoMsg) + Send>` this produces is *not* the
      [rs-fn-field] problem: that is a shared, many-shot `Arc<dyn Fn…>` field,
      while this is one-shot, moved once, and `Send`-checked — which is the
      representational fact that made lambdas-as-targets separable from
      [fate-lambda] and let this ship without it.
    * A free `send fn` itself needs no special emission: it is an ordinary
      `pub fn` whose parameters are all moved and which returns `()`.
  * **[pool-fault-sink] `pool(n, sink)`** →
    `crate::scheduler::salvo_pool_with_sink(((n) as usize), Some((((sink) as usize), |__reason| std::boxed::Box::new(crate::core_actor::__Msg_Faults::Faulted(crate::core_actor::Fault { reason: __reason })))))`. The runtime holds a `String` and cannot
    construct the `Faults` *message* (a generated enum no Salvo fn can name),
    so the *pool creation site* hands over the constructor. Dispatched on
    **arity**, since both `pool` overloads take an `Int` first and the
    intrinsic table's key is the receiver type.
  * **[actor-watch] [actor-on-idle] `Exit` and `Idle` are the scheduler's
    own**: it is Salvo and builds both (`erase(Exit { reason })`), so a
    watcher's payload is an ordinary answer and `watch`/`on_idle` hand it only
    the core's token. What the runtime adds
    for it is an accounting of *undischarged tokens*: `ActorState.owed` counts
    the tokens aimed at an actor, `PoolState.owed` those aimed at a task or held
    by a frame parked on that pool, and `SalvoReply.tracked` is what stops a
    token being counted twice — a delivery clears it, and so does handing the
    token to the scheduler (`watch`, `on_idle`), which is why a
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
    `{ let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Reporting::Reported(label.clone(), out)); __r }`.
    `replyto!` differs only in calling `salvo_mint_gated` — the gate is the
    runtime's business, not the emitter's. The `expect` cannot fire: a parking
    handler may only be spawned ([actor-replyto], checked), so its members run
    as activations and `__addr` was written before the body did.
  * **`k@self(args)`** evaluates the arguments once and picks by `__addr`:
    `{ let __s0 = …; match self.__addr { Some(__a) => <send of crate::__Msg_E::K(__s0)>, None => crate::__Stateful_E::k(self, __s0) } }`
    for a face member (the trait by the handler's statefulness), and the
    `__Priv_H` form with the inherent call `self.k(__s0)` for a private one
    [rs-actor]. One field, both readings [actor-self-send].
  * **The forwarding stub**, `__Stub_E`, beside the effect: a generated IR
    handler (`stub`, constructor parameter `addr`) rendered by the ordinary
    handler path, `#[derive(Clone)] pub struct __Stub_E { addr: usize }` with
    `new(addr)`, implementing `__Stateless_E` by sending
    (`crate::scheduler::salvo_send_wire(self.addr, crate::__Msg_Reporter::Report(what, done), crate::__PROTO_Reporter);`). `use addr` builds one and
    binds it behind the effect's handle exactly as a handler instance is
    bound [rs-handle]. That indifference is the point: a handler is compiled
    once and bound many ways [actor-use-addr]. A spawn clause's addr becomes
    the same stub, behind a handle in the child's fields.
  * [rs-actor-ir] **Which actor code is IR and which is the backend's**
    [actor-msg] [actor-dispatch]. The IR builder generates the enums
    (`Decl::Enum`: `__Msg_E`, one variant per send member and carrying the
    protocol hash; `__Priv_H`, `Init` plus one variant per private send fn;
    `__Cont_H`), the dispatch functions (`__dispatch_H_E`, `__dispatch_priv_H`:
    a switch whose arms `Unpack` the variant and make a `HandlerCall` — a
    face member through its trait with UFCS, `init` and private sends as
    inherent methods) and the stub `__Stub_E`; the backend renders them
    generically, enums at the end of their module. The backend keeps the
    `SalvoActor` body `__Actor_H` (`handle`, `resume`, `decode_reply`,
    `__parked`), the mixed façade `__Fac_H`, the wire codecs and
    `__PROTO_E` (`impl __Wire for __Msg_E`), `__DECODE_H`, and the rendering
    of spawn, send, `replyto` and `waitfor`.
  * **Still refused** (each a diagnostic, none silent): spawning a **generic**
    handler and a **generic effect** as a protocol.

  * `send(reply, v)` without a wire form boxes at the **payload's type**,
    `std::boxed::Box::<T>::new(v)` with `T` read off the token's checked
    payload, so a bare `None` is a typed `Option<T>` rather than an
    `Option<_>` rustc cannot infer.
## Deliberate cuts ([backend-never-wrong])

Reported as codegen errors, never silent wrong code:

* referencing the `Any` type (or any type inference left open) in emitted
  positions: "a value of type `…` reached rust code generation";
* effect member fns with their own generic parameters ("… cannot dispatch
  dynamically yet");
* a generic effect as an actor protocol, and spawning a generic handler;
* a `use` whose effect instance is still generic (an unresolved
  instantiation has no handle type to name, [rs-handle]);
* a mutable use of a value that is a temporary of its own expression;
* a lend or locator shape [rs-loc] cannot express (an intrinsic with no
  locator form, a mutable lend of a non-place);
* a read of mutable data before a later argument mutates it
  [rs-mut-arg-hoist];
* mutating a `Mut` arm of a union parameter received borrowed
  [rs-narrow-mut];
* an intrinsic fn or intrinsic handler with no Rust lowering.

Known acceptable divergences (documented, not errors): extra `.clone()`s
where Kotlin shares references; `Debug`/`Display` formatting of `Option`
values differs from Kotlin's `null` printing (the checker's narrowing rules
make user programs format only unwrapped values); a panic unwinds past code
Kotlin would run in a `finally` [rs-exit-splice].
