# Salvo Compiler — Roadmap

What is **left** to build. Everything finished — the milestones, the four
completed sequences, the options explored and abandoned, and every closed defect
with its repro — is in [COMPLETED.md](COMPLETED.md); this document assumes it and
points into it rather than repeating it.

Consolidated 2026-09-26 (user request): the finished sections that used to fill
this file are gone, their reasoning already recorded in COMPLETED.md's decision
log, and what remains is **one sequence**.

Companion documents: [docs/language/](docs/language/) is the narrative spec
(source of truth); [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md) states every feature as a
labeled rule (`[qual-erasure]` style) with the compiler decisions under it;
`BACKEND_SPEC.<backend>.md` ([kotlin](BACKEND_SPEC.kotlin.md),
[rust](BACKEND_SPEC.rust.md)) repeats rules with backend interpretation and adds
backend-prefixed rules — load one only when working on that backend. Labels are
referenced from code and tests (`grep -rn '[rule-name]'`).

## How to read this

- **The sequence is the order of work.** Take the next unblocked item; nothing
  below step 1 is blocked on anything above it except where it says so.
- **A `DECISION` needs a language-design call before implementation**, and that
  call is the user's: state the options, the trade-offs and a recommendation,
  then wait (AGENTS.md's first invariant).
- **Standing constraints on every item.** Unsupported constructs are *errors*,
  never wrong output [backend-never-wrong]. The two backends must agree
  observably; a divergence is closed by restriction or by faithful emission.
  Backwards compatibility is not a requirement, so a rule change is a sweep of
  every example rather than a shim.
- **Before starting anything**: read COMPLETED.md's "Gotchas / lessons learned"
  for traps in the area you are touching, and its decision log for whether the
  question was already answered.

## Where we are

Everything the four sequences covered is built and running on **both backends
with identical output**: shared fate and borrow emission, linearity with a
designated `close`, effects through handler dependencies, `throw`/`try`, places
and field narrowing, deductions with refinements, the iterator reduction to
`next` (with any-name `iter fn` naming its state struct and `Iter` for
sources), the collections, the filesystem, actors (spawn, send, park, watch,
bridge), time, free concurrency, shareable-by-default handlers, refinement types,
group borrowing (`ref(c)` handles), the testing framework, the comparison/hashing capabilities,
actors across machines (`net`, the whole network sequence), one shape for
effects on both backends (every binding a handle, no fusion, no `local`), and
the first comptime slice (`comptime fn` and `by auto`, replacing `auto`: the
structural `cmp`/`eq`/`hash`/`to_str` are std functions over the fields of any
struct or the arms of any union).
Twelve worked examples in `examples/` carry the checked-in generated code for both
targets and the output they print; the first dependency, `modules/aws/`, keeps
its four (`profile`, `sqs`, `s3`, `ddb`) and its own checks (`modules/aws/check.sh`)
outside the compiler's tests. 711 tests green (nextest); std's own Salvo tests run inside one of them.

## The sequence

### N — A nested-namespace import (left by namespaces, 2026-10-16)

`namespace X { … }` is built (COMPLETED.md, "Namespaces"). A **nested-namespace
import** `import mod.Outer.Inner` may hit
the import splitter's `trailing_upper > 2` cap in `resolve_import` — only
`import mod.X` (whole namespace) and `import mod.X.Thing` (one member) are
exercised. Relax the cap if a case needs importing a nested namespace as a
unit.

### 0 — What the runtime in Salvo left (built 2026-10-03)

Built: see COMPLETED.md's decision log and its "Design record: the runtime in
Salvo" (the runtime record, retired 2026-10-03). What is left, besides 0d, 0e
and the `test actor` follow-ups recorded below:

- **2.** **What is left of the actor, net and time intrinsics** (most went
   2026-10-05 and 2026-10-06, COMPLETED.md; `route_frames`, `watch` and
   `on_idle` are Salvo since §0j 6h, `epoch_nanos` a platform fn since step
   5). §0j schedules `encode`/`decode` as Salvo codecs (step 9); `key_hash<T>`
   is a candidate for a `?hash` implicit after step 8. Still intrinsic, each
   for a reason: `encode`/`decode`, `protocol<E>`, `send(reply, value)` and
   `key_hash` are type-directed (they need a codec or a protocol hash per
   type, which a generic Salvo fn cannot ask for); `pool(size, sink)` and
   `watch_control` hand the scheduler a builder for an actor *message*
   (`Faults.faulted`, the handler's private `control`), which no Salvo fn can
   construct (§0j 6h's DECISION); `thread()` mints a `Dedicated Pool`, a
   provenance claim only a constructor can make; `reply_token` and the four
   handle conversions are the runtime's view of its own handles
   [runtime-handles].
- **6.** **Kotlin's `unerase<T>` cannot check `T`** (erased generics): a mismatch
   surfaces as a `ClassCastException` where the value is used. In the
   runtime a mismatch is a compiler bug either way; an exact check would need
   a type token generated code could pass.

### 0j — Shrinking the backends (user decisions 2026-10-05, partly built)

The aim: a third backend implements a short, straightforward list. Decided in
one sitting; COMPLETED.md's log ("Shrinking the backends") has the decisions
and the survey behind them, with per-site line numbers. Four principles:

- **Ownership is Salvo semantics.** Core computes the ownership decisions; a
  backend renders them, and a garbage-collected backend ignores them.
- **Salvo code over backend code** wherever it can be written in Salvo. The
  scalars stay intrinsic: `cmp`/`eq`/`hash`/`to_str` and the numeric
  conversions on the primitive types, which the operators bottom out in.
- **Salvo semantics never consult host equality, hashing or ordering.** Once
  step 8 lands, backends stop generating host instances. A LANGUAGE_SPEC.md
  rule to write, with a neutral label.
- **A platform fn may keep a fn value that is named, top-level, capture-free
  and effect-free** (the [cmp-carry] identity restriction). Kept closures stay
  the runtime's privilege [platform-fn-value].

The steps, in order. Each says what it absorbs from elsewhere in this file.

- **2.** **Integers**, left of the step built 2026-10-05: the
   `checked_*`/`saturating_*` std functions (§10's overflow row named them).
- **3.** **Shared machinery in `salvo-backend`** — mostly built 2026-10-05
   (COMPLETED.md): `driver.rs` (check, erase, resolve, reach) and
   `emit_util.rs` (23 shared walkers and helpers, `EmittedFile`, ABI
   stamping, file writing). Left: the side-table accessors (`ty_of`,
   `repr_of`, `coercion_of`, … — a shared `CheckedView` both emitters hold);
   one overload-mangling policy (`rust_fn_name` beside `kotlin_fn_name`);
   module selection for ABI mode, which differs slightly between the two
   (`in_abi` against `abi_modules_or_project`) and should be one rule;
   `effect_param_name`/`disambiguate_unchecked` (near-identical; the second
   goes in step 4).
- **4.** **Small core facts**, left of the step built 2026-10-05: one
   termination predicate (Rust `block_terminates`, Kotlin
   `block_returns_early`).
- **5.** **Runtime files and intrinsics to std** — `epoch_nanos` is a platform fn
   with host files `lib/platform/std/time.{kt,rs}` (2026-10-05, COMPLETED.md).
   The runtime files turned out to be used by other runtime code, so each
   moves with what uses it: `hosttime`'s monotonic clock with the scheduler
   (the runtime host files and `scheduler.*` call it; §0d), Kotlin's
   `bytes.kt` (`SalvoBytes`) and Rust's `wire.rs` with step 9 (the codecs and
   the stream table use them), Rust's `seq.rs` (`salvo_pair_mut`) with step 12
   (element handles), `hoststreams` with the stream table's host code. Rust's
   `strings.rs` (float text) is used by the emitter's interpolation; it can
   become a platform fn of `std.core.string` once interpolation lowers through
   `to_str` for floats, which is a small step of its own. The recorded
   runtime-file-name collision shrinks as these go.
- **6.** **The language the Salvo collections need.**
   - b. **`canbe Mut` on a field type** [struct-field-canbe-mut], left:
     Kotlin refuses a `Str canbe Mut` field (a
     `StringBuilder` is not a `String`, so one property cannot hold both
     shapes without `Any` and identity equality) [kt-field-canbe-mut].
   - d. **A total positional write at a field**, left of [col-replace]:
     §0d's `take(place: Mut T?) -> T?`, the same gap at a field.
   - e. **Deep copy of structs on Kotlin** [kt-copy], left: a *generic*
     struct that contains itself, which is still refused.
   - g. **`IntBuffer` and `LongBuffer`** [col-buffer], left: specializing `Int[]` to `IntArray` (M7) stays a
     later, separate question; left out until needed: a fallible `set`, a
     wire form.
   - h. **Platform fns keep named fns** [platform-fn-value], left, a
     **DECISION**: `pool(size, sink)` and `watch_control` build an actor
     *message* (`Faults.faulted`, the handler's private `control`), which is
     generated code no Salvo fn can name, so a kept named fn cannot build it.
     Options: (a) leave the two intrinsic (recommended for now: two small
     arms each, no surface change); (b) a message as a value — a send
     member's message as a fn, `faulted@Faults: (Fault) -> Msg<Faults>`,
     with a new `Msg<E>` type the runtime carries as `Dyn` (a private member
     would still need `@self`, so `watch_control` stays out unless the
     top-level rule bends); (c) change how a sink and a control channel
     receive their payload (e.g. a re-armed reply token per delivery), a std
     API change to `Faults` and the groups.
   - i. **Implicits on stamped fns for generic structs** [comptime-generic],
     left: a stamp with implicits cannot be passed as a fn value (a
     `Set<Wrapper<Str>>`'s `hash`) until step 8. (The generic union with a
     generic struct arm, `type Either<A> = Box<A> | Int : ToStr<self> by
     auto`, works since 2026-10-06.)
- **7.** **The keyed collections** (built 2026-10-06, COMPLETED.md), left:
   - **A `==` on a struct holding a set or map** still compares by the host
     classes' order-blind equality (step 8's host-equality rule).
   - Until step 9, `Set`/`Map` stay `noremote`; their codec is the entries,
     never the slab (hash values differ by backend).
   - The typed override of a same-named implicit (`to_str: (V) -> Str = f`,
     decided 2026-09-28) is refused with a message, not built.
- **8.** **Recursive implicits (§6), then the host-equality rule** (partly built
   2026-10-06, COMPLETED.md), left:
   - **The carried identity as a tree** (decision 3): a `Set<List<Person>>`
     still records the flat `hash@List`, and the element's `hash` is resolved
     where the set is used; the type does not print or compare the levels.
   - **The typed override** of a same-named implicit (`to_str: (V) -> Str =
     f`), refused with a message.
   - **`with` pairing propagating** (decision 4) through a nested fill.
   - **Host equality that remains**: Rust still derives `PartialEq`/`Debug`
     on structs and unions (host code and generated `==` on primitives use
     them), Kotlin's data classes keep `equals`/`hashCode`, and the host
     `Set`/`Map` classes implement order-blind `equals` for the structs that
     hold them. Removing those needs every host-side comparison to go
     through a Salvo fn first.
   - Waiting on it and now unblocked: `Checked<T>`'s `to_str`
     [type-checked], `expect_eq` on a generic container, property testing's
     `?generate`, `json` over container fields.
- **9.** **Wire codecs in Salvo.** `encode`/`decode` (`lib/std/codec.sv`,
   `runtime/routing.sv`) as comptime fns over fields and arms, the `by auto`
   way, on a byte-level platform surface over `Bytes` (write/read `Int`,
   `Long`, `Double`, `Byte`, length-prefixed `Bytes`/`Str`). Deletes
   `wire.kt`/`wire.rs` and every generated codec. Absorbs §2c's "`decode<T>`
   as an implicit": the stamped codec is that implicit. Needs 6i, and 8 for
   container fields.
- **11.** **Effect dispatch and copy plans** (built 2026-10-06), left, small: the dot-notation normalization still asks `effect_of_fn` (step 13
    removes it); the actor-send path (`a.send_fn(..)`) reads its overload
    index off `Callee::Send` since the D2 calls merge.
- **12.** **The ownership plan** (built 2026-10-06, COMPLETED.md). **Open, for
    discussion in TOUR.md §9–10:**
    one new core table, the *demand* on each expression (moved / lent / lent
    mutably / discarded / returned), which is what `ValueMode` is a function of.
- **13.** **The IR** (user direction 2026-10-06; built, steps 1–5 done 2026-10-06,
    record in COMPLETED.md "Design record: the IR"). Both emitters read only
    the IR. **Open:** (a) with the user, revisit whether the actor *codecs*
    belong in the IR (consistency between backends against their tie to the
    `UnionN` layout); (b)
    `BACKEND_SPEC.kotlin.md` still describes several AST-emitter spellings
    and wants a pass rule by rule; (c) a single-face `use` takes a
    `ReentrantLock` it does not need. TOUR.md §10 lists the decisions still
    open.

What a garbage-collected third backend then writes: expression, statement
and type rendering; the `UnionN` generator and union renderers; `try`/throw;
boundary-check rendering; struct, effect and platform-interface rendering;
the scalar intrinsics and arrays; host files for `List`, `Deque`, `Str`,
`Bytes`, `IntBuffer`, time and streams; `copy`; the scheduler's host
primitives. An ownership backend also renders step 12's tables.

### 0d — Shrinking the runtime's platform surface (recorded 2026-10-02)

The user's aim is that a new backend implements as little host code as
possible (runtime record, COMPLETED.md). Some of the runtime's platform declarations are
stand-ins for language features Salvo lacks rather than things only a host
can do; each is a candidate to become Salvo. Kept as they are for now (user
decision 2026-10-02), to be revisited once the port lands. First candidates:

1. **`Slot<T>`** — a cell a linear value is taken from and put back into
   through `Mut`. Stands in for moving a linear value out of a field (gap G2):
   a core `take(place: Mut T?) -> T?` leaving `None` behind, or the checker
   accepting `let b = a.body` followed by `a.body = None` before the
   projection ends. A list position has its total write already
   ([col-replace]); §0j step 6d is this same gap at a field.
2. **`Body`** — an activation's owned, mutable, sendable fn value (E10).
   After the cutover a real actor's body is generated dispatch, so this should
   become what the emitter produces rather than hand-written host code; a
   general owned sendable fn type for programs is a separate decision.

Genuinely the host's, for contrast: `Dyn`'s erasure and downcast, `Parker`,
`start_thread`, `guarded`, `secure_bits`, the clock.

### 0f — Slices: `proj List<T>` and `proj Mut List<T>` (user, 2026-10-04, not built)

A slice is a view of a contiguous range of a list: `proj List<T>` to read,
`proj Mut List<T>` to write through. (**DECISION**, raised by the `ref(c)`
rework, 2026-10-10: a plain `proj` is now read-only whatever its `Mut` says
[ref-handle], so a writable slice would be spelled `ref(list) Mut List<T>`
— a handle whose element is the range — or slices get a qualifier of their
own. The user's call before this is built.) In Salvo it is the existing projection
of the list with a range attached, so it is borrowed from the list, lives no
longer than it, and copies nothing; on Rust it lowers to `&[T]` / `&mut [T]`,
on Kotlin to `subList` (a view there too). What it needs: a slice type the
checker treats as a `List<T>` for reading (`get`, `size`, `iter`, `for`),
`sub_list` answering one, and lifetime emission on Rust for a slice
returned from a generic fn (the gap that made `sub_list` copy).

Functions that would change once slices exist:

- `sub_list(list, from, to)` → answers `proj List<T>` (today a copy, by
  `?copy`); a `Mut List` argument answers `proj Mut List<T>`.
- `partition(list, pick)` → stays a copy (a partition is not contiguous),
  but `split_at(list, i) -> (proj List<T>, proj List<T>)` becomes possible.
- `remove_front`/`remove_back` and the `_while` forms → unchanged (they move
  elements out), but read-only twins `front(list, n)` / `back(list, n)` /
  `front_while` / `back_while` become slices instead of copies.
- `find_first`/`find_last`, `index_of`, `contains`, `any`, `all`, `count`,
  `reverse` → take a slice wherever they take a list, so they work on a range
  without copying it (a slice *is* a list for reading, so this may come
  free).
- `Bytes`'s `slice` and `Str`'s `substr` → the same idea for their own
  types; decide whether they become `proj Bytes` / `proj Str` views too.
- `Deque` → has no contiguous storage on Kotlin (`ArrayDeque` is a ring), so
  it gets no slices.

### 0h — Test speed: prune per declaration (user: "eventually", 2026-10-05)

Measured 2026-10-05: a fresh nextest run is ~6 minutes with all 10 slots busy
throughout (about 3,330 slot-seconds: Kotlin codegen 1,068, Rust 821, CLI
1,374); one Kotlin shard takes 48–54s alone and 185–199s inside the suite.
Most of what each toolchain compiles is std and runtime code the program
never reaches: hello world is 1,516 lines of Kotlin (8 of them `main.kt`) and
2,832 of Rust, because pruning works per module [mod-used-only]. The plan:
extend `salvo_core::reach` to declarations (fns by what the checker
resolved, types by name); runtime files are already emitted only when used
(§0j step 10, built), and §0j steps 5, 7 and 9 also move most runtime files into std,
where per-declaration pruning reaches them). With
[fn-emit-name] std's output no longer depends on the program, so a std edit
would rerun only the programs that reach it. Kotlin already shares one std
compile across programs [kt-std-library] (2026-10-05). Rust does not: it
mounts std as modules of each crate, and two things would also have to stop
depending on the program first — `__loc` locator variants are emitted only
when a caller demands one (forcing them for all of std fails: an `iter fn`'s
lending `next` and `get_present` have no locator lowering), and a file imports
every trait of a module it draws on `as _`, which varies with what is
emitted. Also open: CLI test stamps
keyed by generated content rather than by the `salvo` binary, so a cached run
is a sound pre-commit check (TESTS.md C1, now planned to land with C3/C7). `-XX:TieredStopAtLevel=1` for kotlinc was tried
and rejected (358s against 371s; slower on the large batches).

**The warm run** (measured 2026-10-14, after the corpus tests were stamped):
`cargo test` ~86s and `cargo nextest run` ~80–87s warm, against the ~15s
budget. What is left is not toolchain work but the compiler running inside
the tests: every program a test builds re-checks all of std (~0.15–0.3s
each, 1,000+ programs), and nothing caches it. By binary under `cargo test`:
the Rust codegen tests 18s, the Kotlin codegen tests 16s (its four
compile-and-run shards ~13s each under nextest, building ~50 cases each
before any stamp is consulted), `hygiene_tests` 7s (the `nm` pass over every
artifact), `salvo-cli`'s unit tests 6s (`std_front_expansion_is_project_independent`
7.6s), `platform_tests` 5s (`the_checked_in_host_projects_are_current_and_compile`
~8–10s), `analyze_tests` 5s, `ir_linear_tests` 4s
(`ir_linear_check_has_no_false_positives` ~5–6s), `flow_tests` 4s,
`golden_tests` 3s, `carry_tests` 3s, `lsp_tests` 3s. The levers, none cheap:
(a) under nextest each test still builds the shared std front
(`salvo_testkit::front`, 2026-10-28) in its own process, which TESTS.md T1's
grouping addresses; (b) a shard-level stamp for
the Kotlin compile-and-run shards, keyed by the test binary, `lib/`, every
example's tree *and its resolved dependencies* (`aws = "0.1.0"`), and the
kotlinc version, which would make their warm cost ~0 but needs that input
list to be complete.

### 0i — Leftovers of deterministic names (2026-10-05)

- Rust calls a platform fn's wrapper by its module path
  (`crate::std_core_list::size_platform`) while Kotlin imports it like any fn
  (`sizePlatform`, `sizePlatform__core_list` on a clash): Rust's `__loc`
  locator variants hang off the wrapper's name, which an alias would break.
- A Kotlin predicate qualifier keeps `Q_qualifies` and overloads on the JVM
  parameter type; Rust names it by its subject (`Filled__List_qualifies`).
  Both deterministic; two modules declaring one qualifier over one subject
  would clash on Rust, unhandled.
- `examples/actors` calls `drop(boom)` on an `Int`; `std.core.seq`'s `drop` is not
  emitted there, so Rust's prelude `drop` runs (same effect on an `Int`, and
  it was so under the globs too). Worth a check that a resolved Salvo fn the
  program calls is always emitted.

### 0q — Open defect: moving a field out of an indexed element fails rustc (found 2026-10-20)

`eatl(xs[0].tags)` with an owned `xs: Rec[]` (`=> !xs`) checks clean and the
Rust output is `crate::eatl(xs[(0i32) as usize].tags)`: E0507, "cannot move
out of index of `Vec<Rec>`". Kotlin runs it. A [backend-never-wrong]
violation, older than R8 piece 5 (the flow pass records the move as the
partial move `xs[?].tags` [fate-partial-move], and rightly lets a sibling
read like `xs[1].name` through; the move itself is what rustc refuses). The
fix is the emitter's (a `std::mem::take`/`replace` of the element's field,
or a whole-element move) or the checker's (refuse a move out of an index,
as rustc does), and which one is a language question (DECISION). The
whole element fails the same way: `eatl(xs[0])` of a local
`array_of(Mut List<Str>(), Mut List<Str>())` is E0507 too.

```
struct Rec {
    name: Str,
    tags: Mut List<Str>
}
fn eatl(xs: Mut List<Str>) -> Int => !xs {
    return size(xs)
}
fn idx(xs: Rec[]) -> Int => !xs {
    return eatl(xs[0].tags)
}
```

### 0r — Open defect: an inferred state field read by another inferred one declared earlier (found 2026-10-20)

An `iter fn`'s inferred state field types are interfaces [handler-state-infer]
[decl-interface-pass]: the minter whose state infers one is checked with the
interfaces, before any body, so any *body* reads the type wherever it is
declared (the pipeline record step 7 piece 3). The minters themselves are checked
in declaration order, though, so an inferred initializer that reads another
`iter fn`'s inferred field declared *later* sees `Unknown`, its own field
infers `Unknown`, and a mismatch passes unchecked (a [backend-never-wrong]
hazard: the target compiler is left to refuse it). Repro, no error today:

```
fn peek(f: Firsts) -> Str {
    return f.s                       // an `Int`
}

iter fn firsts(n: Int) -> Emitted Str | Finished {
    state Firsts { s = counts(n).at }
    return finished()
}

iter fn counts(n: Int) -> Emitted Int | Finished {
    state Counts { at = 0 }
    if at >= n { return finished() }
    at = at + 1
    return emitted(at)
}
```

Swapping the two `iter fn`s reports it. The fix (R5 option b's "a cycle is
an error"): check the inferring minters in dependency order (a minter whose
initializer names an iterator struct with an inferred field after that
struct's minter), and report a cycle. Only tests write the inferred form.

Parked (user, 2026-10-14): the user expects to move away from state structs and `iter fn`s altogether.

### 0bb — Leftovers of the pipeline record's stage D (2026-10-30)

- **`erase` is still public** for one reader outside the IR builder, the
  Rust backend's platform-alias expansion (`ir_emit/mod.rs`,
  `ir_emit/decls.rs`): a backend erasing a type itself (D3.4 left it as
  found). The builder's erasure goes through `ir/build/erased.rs` only.
- **`approx_keyed` stays, small**: 16 lowerings a suite run (a variadic
  tail's element type, an `Iter` obligation's element, three signature
  types) where the checker recorded no written type (D3.3(f)).
- **The `@whynot` message** is assembled by the IR from the doc template,
  a presentation question rather than a re-derivation (D3.3).
- **What the round trip still skips** [round-trip]: `for` (its calls are
  only marked in the typed dump), an operator whose operand widens
  [op-promote], qualified types other than `Mut T`; see 0av.

### 0bc — What the generic `NonEmpty` left (2026-10-30)

`NonEmpty` is one `qualifier NonEmpty<T canbe linear> of T` in
`std.core.nonempty`, refined in each type's body [qual-refn-type]
(COMPLETED.md, 2026-10-30). Left over:

- **The "testable where written" rule, not built** (user decision 4.b: only
  as a general rule). The general form: a predicate qualifier whose `of`
  unification fixes all its generics, written over a concrete subject, must
  have its `qualifies` implicits resolvable where it is written — so
  `NonEmpty Int` is refused at the type rather than at the first `is`. What
  building it takes: at `validate_type`, unify the qualifier's `of` with the
  written base, skip a subject that still has type variables, and probe
  `resolve_implicit_value` for each implicit without recording; a new
  diagnostic for the miss (the resolver's are worded for a call), reported
  once per written type.
- **Open defect: a program's same-name qualifier over a particular subject
  now overloads std's generic one instead of shadowing it** [qual-overload],
  and std's type-body refinements, which name the claim, reach it by name.
  Repro: declare `qualifier NonEmpty<T> of List<T> { fn qualifies(list:
  List<T>) -> Bool { return list.size() > 1 } }` (a stricter meaning) and
  `add(xs, 1)` to an empty `Mut List<Int>`: `List`'s `refn add => list:
  +NonEmpty` establishes the program's claim too, which is false for a
  one-element list. Before, the program's `of List<T>` replaced std's (same
  subject). The tests that declared their own `NonEmpty` were renamed
  `Filled`. Fix candidates: let a program's declaration of a std qualifier's
  name shadow it whatever the subject, or key claims by declaration rather
  than by name.
- **A host boundary cannot check a claim whose `qualifies` takes an
  implicit**, so `platform fn names() -> NonEmpty List<Str>` is refused
  (`platform-check`), where the five per-container `NonEmpty`s were checked.
  Building it: resolve the implicits at the platform declaration (the type is
  concrete there), carry them in `BoundaryCheck::Qualifies`, and have both
  backends' boundary code pass them (`kotlin ir_emit/boundary.rs`, `rust
  ir_emit/decls.rs` `boundary_check`).
- (The variadic constructors' copied tail is gone: they consume it now —
  COMPLETED.md, 2026-10-30, "Constructors consume their tail".)

### 0ae — Recorded, not scheduled: per-declaration reuse in the check cache (measured 2026-10-28, A4)

The language server's `CheckCache` [lsp-check-cache] reuses an unedited
*file*'s rows; the edited file is walked whole. Reusing its unedited
declarations' rows too ([decl-rows] gives the partition) was measured at
A4 (debug, `tools/lsp-timing.py`; per-declaration walk costs from
temporary instrumentation of `check_module`): the walk it would save is
`examples/cluster` 4.4ms of bodies over 28 items (the edited `main` is
~1ms of it) against a ~43ms edit; `modules/aws` ~1ms (`s3.sv`'s header cost
is its file-level platform boundary, which a declaration cache would not
reuse); a 1,654-line one-file project (`lib/std/args.sv` as `main.sv`)
16.2ms over 74 items against a ~76ms edit. The costs it adds: inserting the
reused rows (~1.4ms on that file, as the per-file insert costs now) and
moving their spans. Not built, because the gate cases gain less than their
run-to-run spread and what it needs is large:

- every span in a reused row re-derived from the current tree: a key's
  origin (by node id, a per-declaration node → span table built from
  `Module::node_ranges` and the numbering), and the positions inside values
  (`DefSite`, `LocalDecl`, `ImplicitParam::span`,
  `handler_deps`, diagnostics) by the body's growth, as
  `remap` moves another file's definition sites today, and `Span::node`
  ordinals shifted by the edited declaration's node count;
- a file's walk split per declaration: `check_module`'s file-level work
  (renames, case clashes, doc tags, scalar moves, module `use`s, the
  platform boundary, `finish_fate`'s flush) apart from each item's, and per
  declaration step logs for what is order-dependent within a file
  (diagnostics, the ordered tables, `written_types`' if-absent rows,
  `carried_identities`' first writer, ROADMAP 0r's minters).

Worth it when one-file projects of that size are common, or as part of the
typed form (Stage D), whose per-declaration storage makes the second half
moot. On that file the IR (built and analyzed whole every edit, 28ms) costs
more than the walk would save.

### 0ar — Open defects the test-data migration found (2026-10-28, TESTS.md T1)

Moving the checker tests onto the real std and exact annotations (every
diagnostic named) surfaced reports the old tests never looked at. Each case
below stays a Rust test, as it was, until fixed (the file and test named);
repros are over the real std.

- **One mistake, several diagnostics** [type-unknown-lenient]:
  - past a parse error, the recovered AST cascades: `self.note(x)` adds
    `effect-available:own-effect` and `effect-member-call:arg-count`; a
    `[for]`/`refuse!` outside a comptime fn adds the internal
    `comptime-inline:not-expanded` (two parse-only tests kept);
  - also noted, annotated as they are: a non-scalar `NonEmpty` view returned
    owned reports `fate-derived-readonly:view-moved` and
    `fn-syntax:return-mismatch`; a value fitting no union arm reports
    `type-union:no-arm` and the return mismatch; a sourceless `proj` return
    reports `proj-infer:return-no-source` and the IR's `view-moved`.
- **More of the same, from wave 2** (each kept test named):
  - `export linear intrinsic type Handle<T> = Int` reports `type-alias:linear-alias`
    and `linear-opaque:no-discharger` for the refused alias
    (`linear_group_tests::an_alias_cannot_be_declared_linear`, parse-only);
  - two `iter fn`s sharing a state name report `mod-collision:duplicate` and
    `fn-overload-return:duplicate-fn` for the generated `next`
    (`iter_fn_tests::a_state_name_collides_with_a_type`);
  - `when o { is Ok {…} else {…} }` reports `when-union-subject:else` and
    `when-exhaustive:missing-arms` for the arm the `else` meant to cover;
  - `use Echo<Str>(1)` reports `effect-handler-generics:arg-mismatch` beside
    `:conflict` (`implicit_tests::written_type_arguments_must_agree_…`);
  - `list_size@core.list(xs)` on a renamed fn warns `fn-overload-at:not-needed`
    beside `fn-rename:at-on-renamed` (`overload_tests::a_renamed_name_takes_…`);
  - in a parameter type, a local as an identity adds the annotation
    mismatch and warns the local unused; `fn bad<T, ?cmp: …>` reports two
    slot errors; `to_set(List())` reports `call-type-args:cannot-infer`
    twice, for `List`'s `T` and `to_set`'s (`carry_tests`);
  - a linear key (`Set(h)` for a `linear struct Handle`) reports the
    missing `hash` beside the two `linear-generics` refusals and the variadic
    one (`collection_tests::a_linear_element_is_refused_…`); the struct key's
    `eq` is no longer reported after its `hash` (2026-10-30).
- **And from wave 3**:
  - past the parse error, `return v!` adds the return mismatch (parse-only
    test kept);
  - a member call through
    a two-face spawn's tuple reports three errors
    (`multi_effect_tests::several_faces_answer_a_tuple_…`); a cyclic alias
    pair is reported at both declarations and again at a use, and `[any App]`
    adds a no-handler error at the body (two `alias_tests`);
  - `by nowhere` on `: Hashed<self>` reports `obligation-by:names-nothing`
    once per member it would stamp (`hash`, then `eq`);
  - also noted, annotated as they are: a prerequisite cycle is reported once
    per effect on it; a poisoned handle is reported at each use on one line
    (`d.hp = d.hp + 1` gives two).
- **Message text to fix** (the id is right; annotated as it is): a
  dropped `Reply<Int>`'s hint names `on_idle`/`watch` (dischargers listed
  by type name, not instantiation).
- **Behaviour worth a look**: `=> !a` on an `Int` warns
  "changes nothing" yet decides whether the fn fits a keeping `?add`.
- **Smaller**: an effect member with a gained qualifier
  (`fn poke2(s: Mut Store<Int>) [] -> None => s: Stocked Mut`) is not refused
  as a bodiless gain, though `deduce.rs` says it should be (callers do not
  gain it, so no wrong code); a local whose only read is calling it (`n()`)
  warns unused; a servant forwarding a received reply through a dependency
  call (`draw(out)`) needs no `defer`, through a held handle it does.
- **From the run data (TESTS.md C3)**:
  - **The run-data format cannot say** an argument holding a space, an
    environment variable, or a CR on stdin; the tests needing them stay in
    `run_tests.rs` (`run_builds_and_runs_with_each_backend` and
    `build_produces_a_runnable_artifact_…`, which hold those programs since
    2026-10-29).
- **From the second wave (parser and salvo-core tests to ids)**:
  - **a missing diagnostic?** `handler Working() of Work` with no `Work`
    declared reports nothing, in a file whose parse error elsewhere still
    lets `fn-uniform:unknown-fn` through (`check/syntax/expressions.sv#the_receiver_form_is_not_the_self_send`);
  - **inconsistent**: a top-level `refn` reports `qual-refn-scope:top-level-refn`
    only when its qualifier is unsigned, not for the effects, return-type or
    missing-clause shapes;
  - **parse-recovery cascades** (all annotated in `check/syntax/`): `g() is
    "json" | "text"` with `g` undeclared reports `call-resolve:unknown-fn`
    twice; `struct A.B.C` adds `name-dot:not-a-namespace`; `external`/`define`
    declarations recover into checked declarations (`effect-handler-multi:missing-member`
    ×3, `fn-must-return`); a terminal `if … ...` then `elif` gives three
    errors; `"${size("a)} z"` gives three; `let a: [x.type] = p` outside a
    comptime fn adds the unknown type and the return mismatch at `return a`; `is` alone in a
    handler body is called a bare state field.

### 0ba — Recorded, not scheduled: a reference's target by node (D2 navigation refs, 2026-10-30)

`Checked::refs` (`Ref::Def`, `Ref::Field`) names a target as a `DefSite`,
a file and a byte span. When an edit shifts the target in another file,
the check cache moves every such span by the edits in the bodies
(`check_cache::remap_sites`), and gives up the cached rows when one cannot
be moved. A target held as the declaration's (or field's) name node
instead, the way the declaration tables are keyed (`Key::name_of`), would
not move with an edit, and `remap_sites` would go; the language server
would turn the node into a position against the current tree when it
answers. Recorded with the `refs` merge (user, 2026-10-30, item 10), not
built: a field's name node needs its own identity, and the readers
(hover, go-to-definition, rename) compare sites, so they change with it.

### 0as — What member calls and uniform calls left (B3.4, 2026-10-28)

the pipeline record §II.10 P19 is built; these are its recorded leftovers.

- **Two available effects' same-named members** are ranked as one set at an
  implicit position, while a bare *call* still refuses them (D15; left by
  B3.1, whose binder question 0af the user settled 2026-10-30).

- **Member values** (user, 2026-10-28: a future change): `c.bump` without a
  call, as a fn value that sends to `c`, and generally `x.f` read for a
  member of an `eff[E]`. Today a send member is reached only by calling it.
- **A receiver that is not a place** has its member looked up only when no
  function, local or implicit of the name exists, because its type is known
  only by checking it, and it is checked once. So `get(addrs, 0).bump(1)`
  with a `fn bump(c: eff[Counter], n: Int)` in scope is the uniform call, where
  a `let` first makes it the send, with the shadow warning. Fixing it means
  typing the receiver before choosing (a speculative check, or the typed
  pass of the pipeline record's stage C).
- **The shadow warning under `?.`** can only be silenced by writing the
  narrowing out (`if a is T { (a.f)(x) }`): `?.(f)` is not syntax (user's
  option (a)).
- **Sites that read a `Field` callee as receiver-first** without asking which
  call it was: `deduce.rs` `call`, `lends.rs`, `reach.rs`, the fate links and
  the `waitfor` binder inference in `check.rs`. Keyed facts keep a uniform
  call right; a member call through a field could be misread there (none
  found). Piece 5 or stage C should give them the checker's answer.

### 0at — What one selection for members and fns left (B3.5, 2026-10-28)

- **`call-resolve:no-overload` names no candidates**, for fns as before and
  now for members too, where `effect-member-overload:no-fit` named the
  effect (`no overload of `Fs.close` takes (Int)`). A candidate list in the
  message would serve both.

### 0au — What one selection for sends and mints left (B3.7, 2026-10-28)

the pipeline record B3.7 built D18 (a), D24 (a) and the uniform call reaching a
handler's own `send fn`. Cut, each refused with a diagnostic rather than
emitted wrong:

- **A send to an overloaded member of this actor**: a self-send, a servant
  send and a member mint name their member (`SelfSend { member: String }`,
  `ReplyTarget::Member(String)`, the `__Priv_`/`__Cont_` variants and both
  backends' resume tables by plain name), so a name overloaded in the
  handler is refused (`actor-send-fn:overloaded-own`); a handler's private
  sends are still refused overloading at the declaration
  (`actor-mixed-handler:overloaded-send`). Lifting it means carrying the
  member's index through the IR's `SelfSend`/`ReplyTarget::Member` and naming
  the continuation and private variants by `effect_member_name`. An
  overloaded **face** member now has no continuation variant at all (the IR
  and both backends skip it), which is right while no mint can target it.
- **A send's message shape**: defaults, named arguments, implicit and
  variadic parameters on a send member are refused at the send
  (`actor-send-fn:message-shape`): the IR's `Send`/`SelfSend` carry the
  written arguments, and the backends zip them with the parameters. Filling
  them is `args_for` at the send.
- **A mint's ranking is its own** (`free_send_target`): typed captures, the
  token's type from the position, `most_specific`; the call's
  `select_among` cannot take a parameter with no argument yet.

### 0av — What the round-trip check for calls left (B4, 2026-10-28)

the pipeline record B4 built [round-trip] over the examples and the corpus (P5 (b)).

- **What the printer skips**, having no surface spelling (D4 added
  operators, interpolated parts and inferred types, COMPLETED.md
  2026-10-30): `for` (the calls it names are marked in the typed dump,
  not written); calls through a local or a field (an operator or a part
  through the enclosing fn's own implicit too); sends, mints and a call
  whose last argument is a `replyto`; a renamed name; `throw`; a forwarded
  implicit; a type with no source form (qualified other than `Mut T`, a
  borrow, a fn type); an operator whose operand widens [op-promote], since a
  written call widens nothing (P11); desugared or stamped spans.
- **Where it does not run**: 5 of
  the corpus's 6 files do not check (it is the parser's corpus of
  language-page examples: unknown effects, missing type arguments, an
  `intrinsic` outside std), so the round trip runs on `structs.sv` alone,
  which the checker accepts (its use-after-move errors are the IR's
  analyses, which the round trip does not run).

### 0ax — Names still read program-wide (the pipeline record C1.1, 2026-10-29)

C1.1 moved the last program-wide lookups of a *written* name to keys. Two
remain, each a design question rather than a lookup to swap (a third, the
IR's `member_ref_by_name`, went with "Record what each call is in one
table", which reads the member off the checker's call row):

- **Qualifiers are identified by name in types.** A `Ty` carries its
  qualifiers by name (`Qual { name }`), so whatever needs the declaration
  — the IR's `qual_fn_ref` (a `whynot?` or a default's check), the Rust
  boundary check — finds it among every module's qualifiers of that name,
  by subject. [mod-collision] refuses two *visible* qualifiers of one name,
  which is why it holds today; two modules that never see each other, each
  declaring `Q` over one type, would be confused. The fix is a key in `Qual`
  (as `Ty::Named` has), which touches the deduction machinery, keyed by
  qualifier name throughout.
- **Deadlock's inputs name handlers and tasks by written name**
  (`waitfor_sites`, `handler_constructs`, `actor_gates`, `actor_sends`,
  `task_mints`, the graph's node names): two modules' handlers of one name
  are one node. Stage E1 rebuilds those inputs as per-declaration summaries,
  which is where they take ids.

C1.2 and C1.3 (2026-10-29) bound every written type-position and
value-position name where it is written [name-table]. What still goes through
the checker's own scopes:

- **Locals, by name.** The checker's `LocalVar` frames are keyed by name and
  its flow state (narrowing, fate links, captures) follows them; resolution's
  binding for each read is checked against them in a debug build. Stamped
  copies no longer need it (the pipeline record C3.2 made them scopes, 2026-10-30);
  reading by binding is now D2's, when the checker writes the typed form.
- **Unresolved-name diagnostics**, type and value. The checker reports them
  at declaration sites and positions it validates; a complete walk reaches
  more positions (C1.2's differential: group arguments like `: Ordered<self>`,
  names in patterns, value blocks), and a value name's verdict needs what
  only the checker knows: the effects available in the body (members reach
  it through the effect list, an alias or a prerequisite), the handler's own
  `send` members, a façade's confined state. Moving them means stating each
  position first; not scheduled. A qualifier's
declaration is still found as the *first* of its name's overload set where a
subject is not in hand (`written_qual_decl`, `qual_decl`): the generics and
`comptime` flag of a name declared over several subjects are read off
whichever comes first.

### 0an — The `for` header's calls apply no argument contract (B3.3d, 2026-10-28)

Recorded, not a decision: the `next`/`iter` a `for` names are resolved,
ranked, filled and recorded as calls, but what a call does to its argument
places (consumption, the `Mut` mutation of a root, removal sets, derived
links, temporary-view checks) is not applied at them — driving keeps its own
rules for the subject ([iter-drive-in-place], [proj-type], [iter-for-elem-write]).
They agree on every program the suite checks (the differential); folding the
loop's rules into the call's contract would make the header a written
`let it = Mut e; loop { when next(it) … }` (the pipeline record §II.6), and belongs to
the typed partial IR (Stage D).

### 0e — Deferred wakes (recorded 2026-10-03, DECISION, needs a design session)

**What it is.** When a message makes an actor runnable, the scheduler unparks
an idle worker of the actor's pool (an *immediate* wake, today's behaviour).
A *deferred* wake skips that when the sender is itself an activation on the
same pool: the enqueue only counts the item, and when the activation
finishes, its own thread takes one item and wakes one idle worker for each
further one. Measured 2026-10-03 on ping-pong (two actors on one 2-thread
pool, 10⁶ round trips): Rust about 450 → 290 ms, Kotlin about 500 → 300 ms.
With immediate wakes the sender's thread and the woken one race for the one
item, so every message tends to cross threads and pay a park and an unpark.
Fan-out from one actor keeps its parallelism (the wakes happen at the
activation's end).

**The cost.** An activation that sends to its own pool and then blocks
outside a `waitfor` (a slow synchronous platform call) holds that work until
it returns, though a worker is idle; and if it blocks on something only that
work would produce, through host code, it hangs. Salvo code cannot build that
cycle, since its waits go through `waitfor`, which serves the pool.

**Why it was not taken** (user decision 2026-10-03): the scheduling an actor
gets would then depend on which pool it was placed on, which is decided
elsewhere (`spawn … on p`), so the semantics would be hidden from the actor's
implementation. Built, measured, and reverted to immediate wakes.

**The suggestion to evaluate:** deferred wakes, flushed before blocking. The
emitter knows every call to a platform fn or platform handler member; inside
an activation it emits a `flush_wakes()` before each (a no-op when nothing is
deferred), so pure Salvo work keeps the gain and a host call never holds work
back. Open questions for the session: whether a hidden scheduling difference
remains (an activation spinning in pure Salvo for a long time still delays
the work), whether the rule can be stated so an actor author can reason about
it, and the alternative of Go-style timed affinity (wake at once, but a woken
thread leaves the item alone for a few microseconds so its sender can take
it).

### 2 — What the network sequence left (complete 2026-09-27)

COMPLETED.md's log has one entry per step ("The network sequence, step ①" …
"step ⑧"). What the sequence left behind, by step — each a leftover, none a
blocker:

- **① wire/transport**: `delay(node, d)` on `MemNetwork`; a `.test.sv` annex
  for `net` (test bodies have no `spawn`); the multi-face stateful monitor gap,
  which is why the double is two pieces.
- **② codecs**: keyed containers have no wire form; a generic `T` is refused at
  `encode`; Kotlin tuples past `Triple` have no codec.
- **③ routable addrs**: several remote senders may over-subscribe a mailbox by
  one each (the initial grant is "at least one"); a malformed remote answer is
  dropped silently.
- **④ node groups**: the partition/unreachable policy for gossip (a failed
  `deliver` → `left(n, "unreachable")`); the shared secret/TLS half of N-6 at
  the handshake; `HeartbeatNodeGroup` over a `Ddb` platform handler as the
  interop example.
- **⑥ `any`**: an `Addr<E>` answered by *spawning* a router binds as an
  ordinary `E` under `use addr` (the addr type has no room for the claim;
  the deadlock graph prices it since 2026-10-30, every `eff[E]` send landing
  on `E`'s written routers too); a lambda's body is checked under strong
  availabilities.
- **⑦ picks**: consistent hashing for `Sharded` (today `key mod n` over the
  ordered view, so a join reshuffles most keys); a `[keyed E]` claim for a
  program relying on `Sharded`'s per-key ordering; a stub for a group whose
  replica is on another node (empty view, the stub parks); the stub is
  generated only where the module spells the protocol; two instances of an
  erased effect (`RouteSelector<A>`, `RouteSelector<B>`) cannot share a scope — a function per
  policy is the pattern, and lifting it means keeping a phantom on the erased
  trait plus a marker type per effect on the Rust side.
- **the tidy-up (2026-09-27)**: a mixed handler has no `init` block yet. The mixed servant / private-member unification
  (below) is unchanged.
- **⑧ the example**: `examples/cluster/` ships an election by host order behind
  `Leader`, not Raft. **Raft as a flagship example is still to be written** —
  terms, votes, heartbeats over `Timer`, a replicated log — and wants a
  deterministic clock across nodes first (virtual time is per process today,
  and messages between virtual nodes run on other threads).

Both groups stay **actors until the sugar pass** (section 13), which thereby
gains two concrete targets: `members()` as a plain read (the answering stub)
and `replyto` onto another actor's member (the remote mint). Recorded gap: the
deadlock graph is per program, so a wait cycle closing through a handler in
*another* program of the same node group is invisible — the same shape as the
"over types, not instances" gap, same deferred remedy.

### 2c — After `protocol<E>`: fused emission back, effect polymorphism, `decode` as an implicit

The `actor_group`-over-`?protocol` round (COMPLETED.md, 2026-09-28 evening)
left three follow-ups the user has asked for, in this order.

- **Revisit fused effect emission** (user, 2026-09-28: "I'm happy to revisit
  the effect fusion — I personally find it more readable — but after the
  `protocol<E>` work"). One shape for effects chose one handle parameter per effect over the
  fused `__fx` value for uniformity while the shapes were being settled; with
  the handle keyed on statefulness and named after the effect, the question
  is whether a fn declaring `[A, B]` should take one fused parameter again,
  and what that does to `[any E]`, forwarding into spawns, and the deadlock
  graph. The shapes that were deleted, and why, are in COMPLETED.md's
  "One shape for effects" entry — read it before redesigning. **DECISION**
  on the shape before code.
- **Effect polymorphism** — `fn run<T, S, E>(t: T, func: (T) [E] -> S) [E]
  -> S` (user has wanted it for fn-typed parameters; today [fn-effects]
  makes `run` inherit `Logger` from `func: (T) [Logger] -> S`, but the effect
  must be *named*, which is why std's `map`/`filter`/`reduce` take pure fn
  types). Sketch from the 2026-09-28 discussion: a *single* effect variable
  is cheap under one-shape — `run` never calls a member on `E`, it passes the
  handle through, so Rust is `fn run<T, S, E>(e: &E, …, func: impl FnMut(&E,
  T) -> S)` with no trait bound, Kotlin `fun <T, S, E> run(e: E, …)`; the
  checker treats `E` as opaque in the body ([call-resolve]: `run` cannot
  perform it, only hand it on). It must **not** be erased — `Console` and
  `Logger` are different handle structs where every `Addr<E>` is one
  `usize` — so `salvo_core::erase` needs "occurs in an effect list" as a
  reason a generic stays. A *row* (`func` performs several unknown effects)
  is where it gets expensive (no variadic generics on Rust; a bundle struct
  is the fusion shape again), so start with one variable and see whether std
  needs the row — `map(xs, (x) -> println(x))` needs one. To check: `[any E]`
  with `E` a variable; a lambda with an *inferred* effect set against `[E]`
  (bind when the set has one element, refuse naming the row otherwise); the
  deadlock graph prices the concrete instantiation at `run`'s call sites.
  Interacts with the fusion revisit (a fused value is one parameter whatever
  the row), so decide the two together. **DECISION**: single variable first,
  or the row.
- **`decode<T>` as an implicit** — scheduled with §0j step 9: codecs become
  comptime Salvo fns, and the stamped codec is this implicit. The reasoning
  stands — the other intrinsic whose lowering reads
  its type argument, and the same shape as `?hash`/`?eq`: `?decode: (Bytes)
  -> T?` filled by the compiler-derived codec at the concrete call. Worth it
  beyond surface area: today `decode` in a generic body is refused by the
  *emitter* ("needs a concrete type argument"), a [backend-never-wrong] soft
  spot; as an implicit it becomes ordinary colouring ("add `?decode` to your
  signature"), and Kotlin — which cannot monomorphize — gets the codec passed
  in, which is its honest lowering anyway. After it, no user-visible
  intrinsic reads a type argument.

### 2c — Comptime, the rest of the decided design (first slice built 2026-09-28; round 8 built 2026-09-29)

Seven rounds of user decisions (2026-09-28; COMPLETED.md's log, "Comptime, first
slice") and the first slice built the
same day: `comptime fn` with `<T is Struct>`/`<T is Union>` bounds and concrete comptime fns,
`by X` on obligation clauses (structs and `type` declarations) and on fn
declarations, `[for …]`/`[if …]`/`[when …]` (kinds `struct`, `union`,
`tuple`, `fn`, `opaque` = `basic` | `generic`), `v.[field]`, `[field]:` in a
literal, `refuse!`, `T.name`/`field.name`/`field.type`/`field.index`/
`field.first`/`field.last`, `std.core.auto` with `cmp`/`eq`/`hash`/`to_str` for
structs and unions, `auto` deleted, [interp-struct] made opt-in. Round 8
(2026-09-29) respelled it — `comptime fn/struct/type`, `<T is Struct>`,
bracketed heads, `refuse!(…)` — declared the model in `std.core.comptime` as
compile-time-only structs with `Type` a union the kind `when` dispatches over,
and added hover inside comptime fn bodies. What the decided design still owes,
in build order:

- **Obligation clauses on platform types** (decided for `intrinsic type`
  2026-09-28; the collections became platform types and, per §0j, `Set`/`Map`/
  the sorted pair become Salvo structs, whose clauses are ordinary). For
  `List`: `platform type List<T> canbe Mut : Hashed<self>,
  Ordered<self>, ToStr<self>` with the container identities written **in
  Salvo** in `std.core.list` (`fn eq<T>(a: List<T>, b:
  List<T>, ?Eq<T>)`), the scalars keeping their `intrinsic fn`s and declaring
  them (`intrinsic type Int : Ordered<self>, Hashed<self>, ToStr<self>`), and
  `ToStr<self>` on `List` rendering each element with its own `to_str`. Needs
  the relaxation above and recursive implicit resolution (section 6); it is
  §0j step 8. Until
  then the **interim** structural `cmp`/`eq`/`hash` intrinsics over `List` and
  2-/3-tuples in std stand in — they relaxed the recorded "tuple key
  refused by name" limitation on both backends, and an element struct's *own*
  `cmp` is still not consulted inside a list (the Rust backend keeps
  `Hash`/`Ord` derives on structs that have the functions for exactly this).
  Landing it deletes those intrinsics and the conditional derives, which is
  what makes §0j's host-equality rule hold.
- **`by` at a call** (ROADMAP §2c, decided in outline; **DECISIONS** open):
  `Mut Set(hash by auto, eq by auto)` stamps the implicit at the type the call
  binds — a third `by` site, emitted in the calling module once per (comptime
  fn, type), paired by [implicit-with], printed as the stamped root.
  Researched 2026-10-20 (user: research only, not built — COMPLETED.md's
  2026-10-20 comptime entry, design record): today's equivalent is one line
  (`fn my_hash(p: P) -> Long by hash@auto`, then `hash = my_hash`), and
  tuples already have identities in std (`std.core.basic`, `std.core.compare`)
  since 2026-10-06, so the
  feature is a convenience. The decisions it would need:
  1. **The identity it carries into a type.** Identities are names
     (`FnId { name, at }`) compared and printed by name, and a binder later
     calls through whatever the name resolves to in the *reading* module —
     a call-site stamp has no importable name. Options: `hash@auto` (a
     template, not callable), `hash@P` (wrong once `P` has its own), a new
     identity keyed by (comptime fn, type), or a mangled name. Whether
     `Set(hash by auto)` equals `Set(hash = my_hash)` with `my_hash` itself
     `by hash@auto`; whether two modules' stamps of one (fn, type) are one
     identity (only sound if both resolve each field's `hash` alike, and
     [implicit-resolve] is per-module); how a receiving module calls it
     (re-stamp per module, a global synthetic module, carry a declaration
     key, or forbid crossing modules). An identity now names its declaring
     module and resolves there (2026-10-20, [cmp-carry]), which answers the
     receiving-module question for named fns; a call-site stamp has no
     declaring module of its own yet.
  2. **Where the stamp lives**, kept out of the calling module's overload
     set (or it becomes the default `?hash` and collides with a
     hand-written `hash(P)`), its export, and whether it is refused beside an
     existing `hash@P`.
  3. **When it is stamped**: the expansion runs before resolution and knows
     no call-site types. A checker-driven second round (std cache and LSP
     seams), a required written type (`Mut Set<P>(hash by auto)`), or that
     desugared to a hidden `fn … by hash@auto` (the cheapest).
  4. **Generic and non-struct targets**: no tuple template exists; a generic
     struct's stamp fits a keyed slot since 2026-10-20, so a generic stamp
     at a call is a real option.
  5. **[implicit-with] pairing**: whether "stamped" is a source of its own,
     or a group spelling (`Hashed by auto`) fills the whole group at once.
  6. **Typed overrides first**: a callee with two same-named implicits makes
     a bare `hash by auto` ambiguous; and whether it applies at the top level
     only or down a resolution chain (the tree identity of §6).
  7. Printing, diagnostics inside the stamp reported at the call, LSP
     hover/go-to on `auto`, and emission once per (fn, type, module).
- **Typed implicit overrides** (ROADMAP §2c, decided): `to_str(xs,
  to_str: (Person) -> Str = short_name)` — an override keyed by name *and type*,
  consumed at whichever level of a resolution chain it fits (unused: error),
  also the spelling for two same-named implicits at different types; makes the
  carried identity a **tree** printed only where a level is not the canonical
  default (section 6's decision 3). Needs section 6.
- **The `json` module** (`Json` union per SERDE.md SD-1, `to_json`/`from_json`
  comptime fns spelled `by json`, tag rules per SD-4) — waits on SERDE.md's SD-1 and
  SD-4 decisions and on section 6 for container fields.

Known and recorded: two `to_str` overloads that both fit a *narrowed* value
(`to_str(Manual)` beside a stamped `to_str(Source)` where `value: Source` is
narrowed to `Manual`) make `${value}` ambiguous at the interpolation while
`to_str(value)` resolves by [fn-overload-rank]; the comptime fn bodies therefore call
`to_str@[site](value)` rather than interpolating an arm. Pre-existing
[implicit-resolve] locality, not a comptime defect.

**Left by the pipeline record step 6's scoped lookups (2026-10-20).** The expansion
resolves the unstamped program and reads each name through the scope of the
file that wrote it (C3.3a, 2026-10-30, which retired the program-wide
ladder); these stay open:

- **A renamed root** (`import geo.Pt as GPt`, then `[for f in GPt.fields]`)
  is still refused: the world's tables are keyed by declared name, and the
  root test is `contains_key`. Wants the alias → declaration mapping kept in
  the per-file names.
- **The fn reading of `by f` ignores visibility**: a private comptime fn of
  an unimported module is found.
- **`generic_needs` and `group_for` hard-code** member → group
  (`cmp` → `Ordered`, …).
- **Defect, found writing the tests**: `type T = Str | Int : ToStr<self> by
  auto` warns "this code is unreachable" in the stamped `to_str` — `std.core.auto`'s
  union template ends in `return ""` after an exhaustive `[when value]`.
- **The signature/body split** (the pipeline record §5: `by` declares the member
  signatures in 02-resolve-03, bodies stamped in 02-resolve-04) waits on step
  8's stable ids and declaration-relative spans; a stamped member already has
  the signature it is declared with (built 2026-10-14, COMPLETED.md). Left
  beside it: a written `fn … by X` at a generic type is
  still refused (it would have to spell the [comptime-generic] additions
  itself), while the clause stamps one; and `Stamper::member_decl` clears a
  group member's own type parameters (no std group has a generic member, and
  no template for one could be written yet).

### 2d — Fix alongside the `lib/std/args` work: defects found 2026-10-20

Found while designing and building `lib/std/args` (COMPLETED.md, "Design record:
command-line parsing") and the `[if]` scoping fix; the user asked
(2026-10-20) for each to be fixed **in general**, not only where the args
design touches it, and scheduled **alongside the planned `lib/std/args` derive
and comptime work**. None is a decision except where marked. Each was
reproduced in a scratch project; the repros are below.

- **6.** **`?:` on an optional literal union picks one literal (DECISION).** Found
   2026-10-20 building 1.7. The positional pick keeps the subject's first
   *declared* arm [op-elvis], so `parse_output(s) ?: "text"` with
   `parse_output: (Str) -> ("json" | "text")?` picks `"json"` alone and leaves
   `"text" | None` to the right side. But the literals are one runtime arm
   [type-union-arm-identity], so no runtime test can tell `"json"` from `"text"`
   without comparing values, and the right side's `"text"` is typed `Str`
   (the expected type is not passed to it), so the join is `"json" | Str`:
   `let o = Opts { output: parse_output("json") ?: "text" }` is refused with
   "a value of type `"json"` matches multiple arms of `"json" | Str`".
   Repro: that program, with `struct Opts { output: "json" | "text" =
   "text" }`. Options: (a) the pick keeps a **base's literals together**, as
   the runtime arm does (`"json" | "text"` picked, `None` left) — the
   reading the derive needs; (b) keep the declared-arm pick and add a value
   test for a literal arm. Also: pass the expected type to the right side,
   so a literal there is a literal of the union. Recommendation: (a), plus
   the expected type. Until decided, the derive narrows with `!is None`.

- **7.** **A generic caller and callee sharing a type parameter's name misinfer a
   forwarded implicit.** Found 2026-10-20 building `Errs` (1.10). Repro:
   `fn g2<E>(a: NonEmpty List<E>, ?copy: (v: E) -> E) [] -> NonEmpty List<E>
   => !a { return a }` and `fn h2<E>(x: NonEmpty List<E>, ?copy: (v: E) ->
   E) [] -> NonEmpty List<E> => !x { return g2(x) }`: "expected return type
   `NonEmpty List<E>`, found `NonEmpty List<proj ?>`". Renaming `g2`'s
   parameter to `X` compiles. [implicit-infer]'s `learn_from_implicits_once`
   treats the caller's `Var("E")` as the callee's unbound `E` and learns it
   from a visible `copy` instead of the forwarded `?copy`. Fix: substitute
   the callee's generics apart from the caller's before inference, or learn
   from a forwarded own implicit first. `std.core.result`'s `joined` names its
   parameter `X` until then.

- **8.** **A written key differs under the CLI build** (left by
   [type-identity], 2026-10-20), **unexplained**: with the
   wire predicates reading keys through `Checked::written_key`, the CLI
   build (`salvo compile` of `examples/time`, embedded std) judged runtime's
   `WaiterMint { token: Token }` by `args.lex`'s `Token`, while the
   in-process build from disk (`every_examples_checked_in_kotlin_is_current`)
   judged it right. So for that reference `written_key` returned the written
   name under the CLI. Suspects: the std cache seeding `Checked` from
   another tree (address miss), and the by-span fallback
   (`type_ref_span_keys`), keyed by span and name but not file, which drops
   an entry two references disagree on. Emitters read `written_key`, so a
   clashing name in emitted code may be affected the same way. Repro: route
   `wire::approx_ty`'s key through `ctx.checked.written_key` and compile
   `examples/time`.

- **9.** **`?: unreachable!(…)` is not the old `!`** (left by [assert-op],
   2026-10-20): it keeps the subject's **first** value arm,
   where `!` kept every non-`None` arm, so on a union with several value arms
   (`A | B | None`) it is not the old assertion; such sites narrow with
   `assert!(x !is None)` instead. A single expression form that drops only
   `None` does not exist, deliberately (user decision 2026-10-20).

- **9b.** **Open defect: `get(xs, i) ?: "default"` is refused.** Found 2026-10-20
   (item 9). The pick's join is `proj Str | Str`, and the literal on the
   right matches both arms: "a value of type `Str` matches multiple arms of
   `proj Str | Str`; qualify the value to disambiguate", then "no matching
   overload for `println(proj Str | Str)`". Present before item 9 (checked
   against the previous commit's build). Repro: `let xs = List("a")`,
   `println(get(xs, 1) ?: "none")`. A `proj T` and a `T` are one runtime arm
   (`X <: proj X`), so the join should be `proj Str`, with the literal an
   owned value fitting it.

- **9e.** **A general `state` effect (DECISION, to design).** Found 2026-10-20 while
   designing named iterator state (the pipeline record 4.1, "The general form"): a
   `state` statement that brings a struct into scope for the rest of a block,
   like `use` (`state Cursor { src: text, at: 0 }`), and `[state S]` on a fn
   that needs a caller's state, read bare as a step body reads its own.
   Uses: helpers of an `iter fn`'s step, helpers of a handler's members, and
   threading a cursor or machine (a parser), the classic state effect (Koka's
   `state<s>`, Haskell's `State s`). Open: two states of one type in scope
   (refused, as [effect-use-no-dup]); a lambda reaching a state (its type carries
   `[state S]`); states and actors (not shareable); reading the final state
   after the block. The named `iter fn` and handler states (built 2026-10-20,
   [iter-fn] [handler-state-name]; `HandlerDecl.state_name` is recorded and unread)
   are written so that this can extend them.

- **10.** **Template code meets the stamping file's rules** (left by
    [comptime-hygiene]): the same-file and same-module rules a stamped body may meet inside
    template code — [struct-opaque] field access on a template-private
    struct, the `own_qualifiers` constructor rule, the [linear-group]
    same-file discharger — still key on the stamping file, so a derive's
    private *types* would be refused there; `args` keeps its types exported.
    `refine` (refinements per file) and `reach` read the site file's scope.

- **14.** **Kotlin types the emitter writes bare** (left by
    [kt-stdlib-qualified], 2026-10-20): the Kotlin *types* the
    emitter writes bare (`String`, `Int`, `Long`, `Boolean`, `Unit`,
    `MutableList`, `StringBuilder`, `AssertionError`) can be shadowed the same
    way by a program's `struct String` in the same package. Untested; most
    shapes would be a kotlinc error rather than wrong code, but that is not
    established. Qualify them (`kotlin.String`, …) or reserve the names.

### 3 — Recorded: the restrictive reading of a fn-typed slot's lend

Both defects of the 2026-09-25 round are closed — a bare generic struct literal
now determines its type arguments [struct-literal-arg], and a fn-typed slot whose
return is opaque now **infers** its lend [proj-infer-fn-type] (both 2026-09-26,
COMPLETED.md's log).

What the second one left open, deliberately, is the reading it chose *against*:

- **A slot could state that its fn does *not* lend.** `(c: C) -> Mut It` would
  then mean "the iterator must not borrow the container", and implicit resolution
  would refuse `std.core.list`'s borrowing `iter` for it — naming the candidate and
  `holds proj(c)` — while accepting `std.core.set`'s snapshotting one. The machinery
  is already there: resolution **already** compares a candidate's contract against
  a slot's and refuses a mismatch (probed 2026-09-26: a keep/consume mismatch is
  reported with both remedies named), so adding the lend to that comparison is
  small.
- **Why it was not taken**: the restrictive reading buys nothing observable
  today. The two things it would protect — mutating or consuming the container
  while the iterator is live, and the iterator outliving the container — are both accepted
  under *either* reading (probed), and in a generic body there is nothing an
  opaque `C` can be done to anyway. So the restriction would have cost an
  annotation on the language's most ordinary generic-iteration signature to
  express a property nothing yet depends on.
- **The trigger for revisiting**: a program that *needs* an iterator independent of
  its container — one that stores it, returns it past the container's life, or
  sends it to another thread ([actor-sendable] refuses a borrowing iterator, so a
  task or actor taking an iterator is the likely first customer). When that appears,
  the restrictive slot becomes worth stating, and the shape is above. Note the
  matching must be **directional**: a lending slot still accepts a non-lending
  candidate, so it is subtyping on the contract rather than equality.

### 4 — What the project manifest left (built 2026-09-29)

COMPLETED.md's log has the entry. Left behind, none blocking: `salvo lock` as an explicit command was not added
(a build writes the file); the `--target` flag under `backend = "*"` is
per-backend for `run`/`test` (suffixed) and refused for `compile`; only a build
(`compile`, `run`, `test`) reconciles `salvo.lock` — `analyze` and the language
server read the manifest but never write the lock, so an editor session cannot
produce one. Dependencies between projects, parked here as out of scope, got
their first slice on 2026-09-29 ([manifest-deps]: `[dependencies]` + `[build]
modules`, `modules/aws/` consumed by its `examples/profile/`; COMPLETED.md has
the entry). **What that slice left for later**: transitive dependencies (a
dependency's own `[dependencies]` are not followed), version ranges and
resolution, a fetcher that fills `salvo_modules/` from anywhere, a dependency's
own `.svignore` and companions beyond `platform/`.

### 4b — What the first dependency left of the language (decided 2026-09-29)

`modules/aws/DESIGN.md` records the design (user decisions D1–D8 that day):
services generated from the public Smithy models by a `smithy-build` plugin
living in `modules/aws/codegen/`, wrapping aws-sdk-kotlin and aws-sdk-rust through
generated platform handlers (the `smithy-dafny` pattern), non-blocking by taking
a `Reply` and returning at once, streaming bodies through one std effect. The
module is a *user* of Salvo and must not extend the CLI. Its three
AWS-neutral extensions are built; below is what they left, and the
extensions recorded since.

- **1.** **Host dependencies of platform code** (built 2026-09-29), left: a lock
   for host versions, repositories besides Maven Central.
- **2.** **Host-completed continuations** (built 2026-09-29), left: a host reply
   to a remote-minted token (the typed wire path), and detecting a lost
   reply on Kotlin.
- **3.** **`std.stream`** (built 2026-09-29), left: the writer pair for Salvo-produced bodies a host consumes,
   asynchronous `Lines`, a consumer-only face, and `MemStreams` for a test with
   no filesystem (DESIGN §9).
- **4.** **Provider-checked stream handles** (DESIGN §9, decision 10 of 2026-09-29's
   second sitting). A `MemFs` stream handed to `HostByteSource` traps at
   runtime; the user wants the compiler to see it. Sketch to start from: a
   handle carries its **domain** as a provenance qualifier (`Host InStream`,
   `Mem InStream`), each handler declares the domain its handles live in, and
   at a `use` the effects in scope must agree on one — an error at the binding,
   not at the read. **DECISION** at implementation time: qualifier vs. a type
   parameter on the effect vs. a property of the handler declaration, and how a
   program that legitimately mixes domains says so. Not blocking §4b 1–3.
- **6.** **Handler bundles** (recorded 2026-09-29, **DECISION**, not scheduled). A
   production composition root binds a whole collection of handlers, and the
   stream layering makes even a host filesystem four lines (`use
   HostRawStreams()`, `use DefaultStreams()`, `use HostRawFs()`, `use
   DefaultFs()`). A named bundle — one declaration that binds several handlers
   in dependency order — would make that one `use`. First customer: streams +
   fs; the shape (a declaration form, a fn that `use`s, how a bundle composes
   with `with` and interception) is the user's call when it is picked up.
- **7.** **Program end on the JVM** (found 2026-09-30, **DECISION**: to be designed
   fully before building). A Rust program ends when `main` returns; a Kotlin
   program ends when the JVM's last non-daemon thread does, so a host library
   holding one keeps the process alive after `main` — OkHttp, under
   aws-sdk-kotlin, for 60s after its last call. The stopgap is in the aws glue
   (`salvoCloseWhenMainEnds`: a daemon thread joins `main` and closes the SDK
   client). The general fix is the Kotlin runtime ending the process when the
   Salvo program is done (`exitProcess` after `main`). To settle: what "done"
   means when actors, spawned pools or host-completed replies are still
   outstanding (today's rule on each backend, and whether the two agree);
   whether host code gets a shutdown hook (closing clients, flushing) before
   the exit; the exit status (a fault that reached the sink, a trap); and the
   cost — every emitted `main` changes, so every golden and example tree.

Outside the compiler, the `aws` module (COMPLETED.md has its entries) left
(DESIGN §9): multipart upload for a body of unknown length, Smithy unions,
paginators and waiters, more S3 operations (buckets, listing).

### 4c — What unions of literals left (built 2026-09-30)

`Byte` literals, which need literal syntax first [type-literal].

### 5 — Consistency passes the 2026-09-26 ambiguity round left

Two narrower questions, both downstream of "refuse to choose" (COMPLETED.md's
log for the round itself).

- **A `@mod` selector for "this file's module"?** Identical-signature
  shadowing stays an ambiguity (user, 2026-09-26), and its remedy is
  `f@<this module's path>(…)`, and there is no shorthand — `@mod` does not
  exist, and `@mod`/`@import` as *rung* keywords were rejected 2026-09-07
  because they ask the reader to know which rung a name arrived on. A `@mod`
  (or `@self`) meaning **"this file's module"** is narrower than what was
  rejected: one fixed meaning, no rung to know, and it stays short where a
  module path does not (`size@orders.pricing(xs)`). Options: leave it (the path
  is explicit and greppable, and a call needing the selector is rare by
  design); add `@mod`; or add `@self`, which already means "this handler" for a
  send [actor-self-send] and would then mean two things. Recommendation: leave
  it until a real program reads badly — the trigger is narrow enough that the
  evidence may never arrive.
- **The rest of the resolution-by-position rules.** The round covered calls and
  refinements, and implicit resolution followed on 2026-10-14 (it is a call's
  rule now, COMPLETED.md). A pass over the spec should list every remaining
  resolution-by-position rule and decide each — separating *silent winners* from
  *designed shadowing* (a later `use` shadowing an earlier handler is interception
  semantics [effect-intercept], and a fn-typed local shadowing a name outright is
  the caller's explicit choice). Known still to walk the rung ladder, found
  2026-10-14: a `replyto`/mint's `send fn` target (`free_send_target`); a written
  identity's fallback when resolution cannot decide (`written_declaration`);
  and which candidate *leads* the arguments' expected types at a call (the top
  rung, [fn-overload-rank]'s "lead", a hint only).

### 6 — Recursive implicit resolution (decided 2026-09-28, not built)

[implicit-resolve] **skips a candidate that itself needs implicits**, so
`fn eq<T>(a: List<T>, b: List<T>, ?Eq<T>)` can never be what a `Set<List<Person>>`
resolves, and the container identities have to be structural host intrinsics
(the interim ones std carried since 2026-09-28, see section 2c). The
lift has two halves: resolution — `resolve_implicit_fn_at`'s one-line skip
becomes a recursive resolution — and emission — a filled implicit that itself
needs implicits is handed its own, so `ImplicitArg::Resolved` needs nested
arguments and both backends' adapter closures pass them. The second is the
larger.

**The six decisions, all made** (user, 2026-09-28, comptime rounds 3–7;
COMPLETED.md's comptime entry holds the argument):

1. A candidate whose own implicits cannot be filled is **an error naming the
   chain** (`cmp for (Int, Foo) needs cmp for Foo: none in scope`) — never a
   silent non-candidate (SFINAE). (The "unless another candidate resolves
   fully, in which case that one wins" of 2026-09-28 is superseded: since
   2026-10-14 the candidate is chosen first, by a call's rule, and only its
   implicits are filled.)
2. **Depth cap 8**, worded "it nests more than 8 levels deep (the `cmp` for
   `?cmp: …` needs implicits of its own)", with the chain and "pass `cmp = …`
   explicitly"; a chain that needs itself is refused at once as a cycle, with
   the chain printed (the message's shape: [implicit-recursive], 2026-10-14).
3. **The carried identity is a tree** [cmp-carry], compared as one — two sets
   over `(Int, Person)` keyed by different `Person` hashes are different types —
   and **printed only where a level is not the canonical default**, so
   `Set<List<Person>>` prints bare in the ordinary case and as
   `Set<List<Person>>(hash@List(id_hash), eq@List(same_id))` when the user
   wrote the leaf. (Round 6 reversed the earlier "canonical by construction"
   choice once the typed override below could produce a non-canonical leaf.)
4. **`with` pairing propagates** [implicit-with]: `Hashed<(A, B)>` fills
   `Hashed<A>` as a pair; half a pair at any level is the existing error.
5. **std owns container identity**, in Salvo: obligation clauses on the
   platform type `List` (section 2c) and ordinary Salvo instances on the
   `Set`/`Map`/sorted platform types; the interim intrinsics and the Rust
   backend's conditional derives are deleted when it lands, and `xs == ys` on
   two `List<Person>` then consults `Person`'s declared `eq`.
6. Recursion applies **everywhere resolution runs**: calls, picks,
   interpolation's `to_str` lookup, and the copies inside a comptime fn.

Waiting on it: `Checked<T>`'s `to_str` [type-checked], `expect_eq` on a generic
container [interp-to-str], property testing's `?generate`, section 2c's
`intrinsic type` clauses, typed overrides and `json` over container fields.

### 7 — Qualifiers are droppable, then variance

- **Open defect: a qualifier dropped inside a branch that narrowed the same
  variable comes back at the branch's end** (found 2026-10-30 building
  [qual-retype], not reproduced as wrong code): `with_narrows` restores a
  narrowed root's flow type when the branch closes, whatever the branch did to
  it, so in `if x is NonEmpty { x := -Mut }` (or a call there whose clause
  drops `Mut`) `x` is `Mut` again after the `if`. Regaining a claim the value
  still has is sound for a re-typing (it writes nothing); for a call that
  mutated it, the claim it dropped is restored too. The fix is a join of the
  branch's exit state with the restored one rather than a plain restore.
- **An inferred type argument is not widened by the expected type** — found
  while building step 1(b), and the same invariance in a narrower place.
  `checked<T>(value: T)` binds `T` from the argument, so
  `checked(NotFound { … })` is a `Checked<NotFound>` and the expected
  `Checked<FsError>` does not widen it — not through a `let` annotation either
  (probed). std therefore names the argument at all 35 construction sites,
  `checked<FsError>(NotFound { … })`, which is explicit and costs a word. The
  declared-position widening that the old `FsError { kind: … }` relied on still
  works; what does not is *seeding a type variable* from the expectation. The
  keyed-container work seeds a callee's type parameters from the expected type
  already, so the machinery is there — the question is whether an expectation
  may pick a **supertype** for an inferred argument, which is the same question
  variance asks and should be answered with it. **A small DECISION**: widen
  (unify against the expectation's arms, so `checked(NotFound { … })` at a
  `Checked<FsError>` position infers `FsError`), or leave the explicit form as
  the one way and keep this recorded as intended.
- **DECISION — variance on generic parameters** (user direction 2026-09-23). A
  `List<NonEmpty List<Int>>` is not a `List<List<Int>>` today, so a constructor
  call cannot fill a plain type-argument position. The direction is `in`/`out`
  declaration-site variance as C# and Kotlin have it. What has to be decided with
  it: declaration-site or use-site (recommendation: declaration-site first,
  because std's containers are where it pays); what `Mut` does to it (a
  `Mut List<T>` cannot be covariant in `T` — the classic array-store hole — so the
  natural rule is that covariance holds only while the value is not `Mut`);
  whether qualifiers on a type argument are a **separate, narrower** rule worth
  pricing first (the case that raised this needs only `NonEmpty List<Int>` →
  `List<Int>` *inside* a type argument, which is qualifier-dropping at depth); and
  that the emitted Rust must not depend on it, since Rust has no variance.
- **DECISION — the mirror of [fn-kept-borrow]** (found 2026-10-14): should a fn
  whose parameter is a borrow (`fn longer(s: proj Str) -> Bool`) fit a slot
  that passes an owned value (`(Str) -> Bool`, e.g. `any(ws, longer)` over a
  `List<Str>`)? The caller would lend the value for the call, which is safe: a
  `proj` parameter cannot outlive the call, and the caller keeps ownership. On
  Rust it needs an adapter passing `&x` (the reverse of the one
  [fn-kept-borrow] uses); Kotlin needs nothing. Options: (a) allow it, for
  symmetry with [fn-kept-borrow]; (b) keep refusing it, the remedy being a
  lambda (`(s) -> longer(s)`). Agent's lean: (a).
- **Open defect: a `Map` keyed by borrows cannot be built from borrowed
  values** (noticed 2026-10-14, not investigated): building a `Map<proj Str,
  …>` from tuples whose `proj Str` component comes from a view (`("ab", h)`
  style entries over a `List<proj Str>`) is refused with "cannot store a
  projection", though a `Set<proj Str>` and a `Map<proj Str, V>`'s lookups now
  resolve their implicits ([fn-kept-borrow]). Find whether the refusal is the
  [proj-type] store rule applying where the container itself is a view (then
  the rule should let a view container store views of a source that outlives
  it), or a missing case.
- **Open defect: `SortedSet(...vs)` over a view list finds no overload**
  (noticed 2026-10-14, not investigated): spreading a `List<proj Str>` into
  `SortedSet(...)` is "no matching overload", while the same over a
  `List<Str>` works and `cmp` of `Str` now fills a `proj Str` position
  ([fn-kept-borrow]). Likely the spread constructor's element type is matched
  invariantly against `proj Str`; reproduce, then decide whether it is the
  constructor's signature or the spread's typing.

### 8 — Mutating through a union arm (DECISION)

One shape is **refused on Rust and accepted on Kotlin**, which is a divergence
closed by restriction on one side and therefore a decision rather than a resting
place:

```
fn bump(o: Ok Mut List<Int> | Err Str) [] -> None {
    if o is Ok {
        add(o, 7)          // Kotlin: mutates the caller's list
    }                      // Rust: reported — `o` is read-only here
}
```

The parameter renders `&Union2<…>` because an arm's `Mut` is not a claim about
`o`, and **nothing can ask for the `&mut`**: a written `=> o: Mut` is refused
("a deduction may preserve or drop qualifiers, not add them"). So the question is
what a `Mut` arm means for the *parameter* that carries it.

- **(a) The arm's `Mut` makes the parameter mutable.** Cost: the Rust signature
  then disagrees with the checker's contract, which still says the parameter is a
  kept read — and two arguments naming the same place, which the checker permits
  as two reads, become two `&mut` borrows and an E0499 on a program Salvo
  accepted. Teaching the fate analysis about that is the real scope.
- **(b) Admit the deduction.** Relax "may not add qualifiers" so `=> o: Mut` is
  legal when `Mut` is present on an arm, and let inference write it from the body.
  Cost: a written deduction now means "through an arm", a new reading of the
  clause; benefit: the contract stays visible in the signature, which is what
  deductions are for.
- **(c) Keep the refusal, and refuse it in the *checker*** so both backends say
  the same thing. Cost: a Kotlin program stops compiling; benefit: one story, and
  the remedy (take the payload as its own `Mut List<T>` parameter) is one line.

**Recommendation: (b)**, with (c) as the fallback. (a) is the one to avoid: it
makes the two sides of the compiler disagree about what a signature means, which
is how the original defect happened.

### 9 — Testing, beyond the MVP (decided 2026-09-23, not built)

The framework's core is built and `salvo test` runs std's own suite. What was
deliberately cut, in the order the decisions put it:

- **`context` scopes** — decided in shape (TF-9): nesting plus per-test re-run
  initialization (`use`s and `let`s that run again for every test, so no
  cross-test state is expressible), with the test's id joining the context names.
  Three sub-calls ride the implementation: statements **preamble-only**
  (recommended, error otherwise), a preamble may do **anything a test body may**,
  and a **childless context warns**. The lowering is per-test inlining, which
  makes isolation true by construction.
- **Property testing** (TF-5): `check(runs, property)` with the generator arriving
  as the implicit **`?generate`**, so the *qualifier on the parameter type picks
  the generator* (`(text: ValidDate Str) -> …`); randomness threaded as a
  `Mut Rng` **value**, which the effect-free-resolution rule turns into a
  determinism guarantee; shrinking by replaying the generator over a shrunken draw
  stream. Two prerequisites: wrapping and bit operations on `Long` for a
  pure-Salvo splitmix64 (parity by construction; now §0j step 2, as
  operators), and step 5's implicit lift.
- **Actor testing helpers** (TF-6): `settle(p)`/`expect_settled(p)` over
  [actor-on-idle], a recording `Probe<M>` handler, `expect_fault(target, body)`
  over [actor-watch]. Needs `spawn` added to [test-body]'s implicit powers, and
  the harness's default fault sink to record rather than print. No test scheduler
  (rejected as out of scale).
- **The blackbox tier** (TF-2's `tests/` tree): ordinary modules that `import`
  what they test and see only exports.
- **`--isolate`, `--timeout`, crash recovery** (TF-4): one process per test, a
  per-test wall-clock kill, automatic re-run of a crashed batch's remainder. The
  MVP runs everything in one process and reports a test that died (`DIED`).
- **Smaller leftovers**: the annex's one-way visibility holds by construction but
  is not *checked*; the report has no `--format json` for editors; a test's stderr
  is attributed by arrival [test-output], so a line written just before a
  verdict can land under the next test; `Console` has no unbuffered `read` (a
  chunk, or the rest of standard input) beside `readln` [std-console] — asked
  2026-10-20, not decided; nothing
  migrates the compiler's own e2e suite onto `salvo test` (user decision: leave
  it).

### 10 — The assertion trap policy (A-6, decided 2026-09-23, not built)

One failure class still takes the hosts' behaviour (a subscript out of range
traps with Salvo's message since 2026-10-30, and integers wrap [op-wrap]):

- **Division by zero** → trap with our message; both hosts already trap, only the
  text differs.

### 11 — One read, one mode: the rendering that reports a reference

Folded into §0j step 12 (2026-10-05): the ownership plan's per-read table
tells each site what a read produces, which is the information this section
asks the rendering to report. The record below stays as that step's first
customer.

Three slices landed 2026-09-23 [rs-read-mode]; what is left is the **refactor the
section is named after**. The slices work because their sites know the shape they
will get and can ask a predicate first (`owned_optional_local`,
`narrowed_borrow`). The general case cannot: a site that wants `&T` writes
`&{code}`, and if `{code}` is already a `&T` the result is `&&T`. The durable
answer is for the rendering to answer *what it produced* (`Rendered { code,
is_ref }`) so a site can decide whether to add the `&`.

The site waiting on it is **interpolation**, the most common read position in the
tree, which still clones every narrowed or `!`-ed value:

```rust
// lib/std/test.sv's expect_trap_with, where the same `trap` two lines up borrows
format!("… got `{}`: {}", trap.as_ref().unwrap().clone(), label.clone())
```

`emit_interp_value` cannot simply switch to `emit_read`: its native branch would
take the reference happily (`format!("{}", &String)` displays), but the two
`to_str` branches in the same function write `to_str(&{arg})` and
`{place}.{field}` — three questions to the predicate in one function, which is
precisely the shape that wants the rendering to answer for itself.

Worth finishing because it is the last **systematic** copy nobody wrote,
`[copy-opt-in]` is a stated principle rather than an aspiration, and it is
measurable: `examples/*/rust/**` carries the clones, so the diff *is* the
benefit. Two recorded items are the same missing information in other clothes:
the temporary-subject `for` loop, and the copy an adapter closure makes of a
returned projection.

### 12 — Effect transformers (E3 step 4)

The last rung of the handler-control arc. A **transformer** is an effect member
that runs a fn-typed parameter with *additional* effects available — its body
having registered handlers for them. `Retry`, `Timeout` and async are all that
shape, and `try` could be re-expressed as a library transformer if it reads
better than the intrinsic.

- **The gate is already built**: step 3 made fn-type effect lists real and threads
  effects *into* a fn value instead of capturing them [fn-effects]. What remains is
  the surface.
- **No silent colouring** (user decision 2026-09-04): the ability to not resume is
  declared on the effect member, never discovered from the handler.
- **Async is explicitly not part of this arc**: it arrives later as an explicit
  effect, likely a compiler intrinsic, not as an `async`/`suspend` transform of
  the whole program.
- **Multi-shot resumption is closed on principle**: resuming twice duplicates a
  use obligation, so it cannot coexist with `Linear`. (Neither target offers it
  either.)

Two related recorded items: a **handler cannot dispatch to itself** — inside a
member the bare member name is already taken (it means the handler registered
*before* this one [effect-intercept]), so self-dispatch needs a different
spelling, and that spelling is a language call (`self.bump()`, `bump@self()`, …);
the workaround (a free fn both members call) is what std does and has cost
nothing. And **two cuts inside the effect fusion** stay *reported* rather than
mis-emitted [rs-effect-fusion]: a dependent handler using its own generic
parameters in a member signature, and a `use` whose effect instance is still
generic.

### 13 — The sugar pass (after the explicit surface, decided 2026-09-15)

Phase 5 delivered the **explicit** actor surface (tokens and reply parameters
written out); every layer of sugar above it is a later item with its own decision
surface. What is already decided, so the pass starts from a plan:

- **Per-kind `-> T`** (EU-6): plain effects unchanged forever; inside an
  `actor effect`, `fn m(a) -> T` means an implicit trailing `Reply<T>`,
  fulfil-at-every-return, and caller-side call syntax = auto-mint + gate. This is
  why `send fn` stayed explicit: the non-send forms are stated *against* it.
- **The generalized mint** (EU-7b): `replyto k(c)` resolves lexically against the
  enclosing handler, else through the effect list as a *remote* mint — a curried,
  capacity-reserved, one-shot send, with reservation at mint time so discharge
  never blocks, and mint sites contributing deadlock edges like sends.
- **Two stub readings appear here, and only here.** An answering member cannot
  have one implementation for both bindings: from an actor the call parks, from
  synchronous code it must block.
- Then the rest of the tower, each its own call: `then`/`then!`, `defer`,
  merge/join, the gate's member-set generalization.
- [fate-lambda] belongs here — no first-pass form crosses a closure. The recorded
  refinement is `move`-closure emission with hoisted clones, plus a treatment for
  captured effect-handler locals.

### 14 — Shared mutable state: `Cell` (DECISION)

Deferred until after actors deliberately, because the OTP answer is that actors
own their state and message-pass — which may remove the motivation. The full
design (a capability *qualifier* rather than a container type, the representation
per backend, why it cannot panic, the rules it drags in, and the producer case
that is its hardest customer) is recorded in COMPLETED.md's "Shared mutable state
(`Cell`)" section, moved there with this consolidation. The question to answer
first is whether shared mutable state joins the language at all.

When it is taken, it should be decided **on one table** with the rest of the
sharing story: shared-immutable versus shared-mutable, invalidation-checked
versus unchecked (today's fate links, frozen `Reg`, `canbe` groups, `Cell`) —
with the observation that `proj` is the degenerate group (a read-only member of a
singleton group under maximal invalidation sensitivity), so the two are points on
one dial rather than two features.

### 15 — `ref(c)`: what is left

The `ref(c)` handle design is built through to element containers and
unions of borrows (2026-10-10, COMPLETED.md's "Handles are storage paths"
entries and its "Design record: group borrowing", which retired
GROUP_BORROWING.md; [rs-path], [ref-anchor], [ref-notsame],
[fate-elem-distinct], [proj-opt-slot], [rs-opt-borrow]). What is left:

0. **Follow-ups to "positions are preserved, never recovered"** (landed
   2026-10-10, COMPLETED.md's entry of that name; to discuss, not scheduled):
   (a) *the two-function choice* — a direct-borrow face for read-only uses of
   a `ref`-returning function on Rust, as a backend optimization once
   measured (today a read-only use re-walks the path); (b) whether `ref`
   merges into `proj Mut` now that the relation is one-way and permission
   follows the kind; (c) an opt-in slot-map container if recursive types or
   performance ever need O(1) handles (no region over ordinary containers:
   see the entry for why). Std's `at` was widened from `List<Mut T>` to any
   element (permission follows `T`'s kind); the user confirmed the API
   shape.
1. **Recursive containers** (a type holding its own element type) have no
   finite path: refused, and waiting on the recursive-types work (item 17).
2. **Open defect: a read through a field of a nested `get` fails rustc**
   (found 2026-10-10). The Rust emitter clones the field into a temporary
   and borrows from it, which rustc refuses (E0597) — loud, not silent:

   ```
   struct Track { plays: Int }
   struct Album { tracks: List<Track> }
   let albums: List<Album> = List(Album { tracks: List(Track { plays: 1 }) })
   println("${get(get(albums, 0)!.tracks, 0)!.plays}")   // rustc: `__tmp1` does not live long enough
   ```

   Binding the outer `get` first works. The fix is a borrow of the field
   (`&__proj.tracks`), not a clone of it.
3. **Multi-arm `Mut` unions have rough edges in the checker** (found
   2026-10-10, while testing [proj-opt-slot]): `=> x: Mut` is refused on a
   `Mut A | Mut B` parameter ("deduction keeps qualifier `Mut`, which is
   not declared"), and the calls after it then report `no matching
   overload for bump(Mut (Mut A | Mut B))` — the qualifier wrapped around
   the union a second time. Writing `=> x: None` works. `Mut T?` and the
   `Ok Mut X | Err E` shape need no workaround.
4. **Type arguments are not inferred from an in-scope implicit** (found
   2026-10-10). Inside `fn bump_at<C, L>(c: C, l: L, ?Ref<C, L, Counter>)`,
   calling `fn outer_at<C, L, T>(c: C, l: L, ?Ref<C, L, T>) -> ref(c) Mut T?`
   as `outer_at(c, l)` reports "cannot infer type argument `T`": `T`
   appears only in the implicit and the return type, and the in-scope
   `?Ref<C, L, Counter>` that fills it is not consulted.
   `outer_at<C, L, Counter>(c, l)` works. Inferring from the implicit that
   fills the group would remove the annotation.
5. **Overload ranking prefers a wrap over an exact fit** (found 2026-10-10).
   With `fn f<T>(x: T)` and `fn f<T>(x: T?)`, `f("a")` picks the optional
   form: by signature `T?` is the more specific pattern. An exact-fit-first
   rule (drop candidates that must wrap a present argument into an optional
   when another fits as it is) fixes it; prototyped and reverted, since it
   is a ranking rule — **DECISION**.
6. **Open defect: a read-only loop over a `List<Mut T>` field borrows
   mutably on Rust** (found 2026-10-10, predates the `ref` work):

   ```
   struct Player canbe Mut { goals: Int }
   struct League canbe Mut { bench: List<Mut Player> }
   fn most_goals(l: League) -> Int => l {
       let best = -1
       for p in l.bench { if p.goals > best { best = p.goals } }
       return best
   }   // rustc E0596: `for mut p in (&mut l.bench).iter_mut()` over `&League`
   ```

   The loop element is typed as a handle (`ref … Mut`), so `vec_iter` picks
   `iter_mut`; whether it is written should come from per-binding mutation
   inference ([rs-elem-mut]).
7. **Latent defect: `walk_pair` over a `VecDeque`** (found 2026-10-10): the
   generated `Walk` impls split with `salvo_pair_mut(&mut c[..], …)`, which
   a `VecDeque` cannot take; rustc errors once a program has `Deque` handles
   and a fn generic over a handle's container. `make_contiguous()` (indices
   are what paths hold) would fix it.

### 16 — Composing iterators: stages over a generic source (after the redesign)

The iterator redesign (COMPLETED.md, 2026-09-27) settled §16's *spelling*: a
stage is an `iter fn`. Over a **concrete** source it works today —

```
iter fn evens(xs: List<Int>) -> Emitted Int | Finished {
    state Evens { inner: Mut ListIter<Int> = iter(xs) }
    …
}
```

— nothing is minted until the call, per drive, so replay is free, and
`for n in doubled(evens(xs))` chains. What is **not built** is the generic
stage, in either of its two forms, both refused today at the `iter fn`:

- **over a generic source**: `iter fn evens<C, It>(c: C, ?Iter<C, It, Int>)`
  with `state Evens { inner: Mut It = iter(c) }`;
- **over an iterator**: `iter fn doubled<It>(it: Mut It, ?Next<It, Int>)`,
  the position *consumed* into the struct (today every `iter fn` parameter is
  borrowed and read-only).

Both need the same two things (ITER_REDUX.md, round 6): the generated `next`
carries the **forwarded implicits** (`?Next<It, Int>` on the hidden `next`,
forwarded by whoever drives it), and the [group-obligation] match **ignores
trailing implicit parameters** when checking the hidden struct's `: Next<self,
T>` — implicits are the callee's business, resolved at the call, not part of
the member's shape. The alternative is storing the inner `next` as a fn-valued
field, which nothing lowers since §0j step 7 removed the stored-implicit
convention for keyed containers, so the implicit-on-`next` form is the one. A hand-written generic stage (`struct
MapIter<It, T, U> : Next<self, U>` with `next(p, ?Next<It, T>)`) hits the same
obligation gap today, so this is a prerequisite rather than a cost of the sugar.

What falls out once it lands: pipelines as values (`let pipeline = c ->
doubled(evens(c))`, passed as `total(xs, iter = pipeline)`); `zip`, `chain`,
`take` as `iter fn`s with iterator parameters and no hand-written struct; the
two-sources case (`?Iter<A, T>, ?Iter<B, T>` — two implicits named `iter` and
two named `next` at different types) as the first thing to test.

What stays parked: a stage over a **linear** iterator (`Lines`). The hidden
struct would have to be `linear` with a discharger, and the user decided
(2026-09-27) no discharger is generated — a discharger would have to be a
member of the group to be consistent — so that iterator is written by hand,
and generic code that may receive one takes the discharger as a callback
[linear-generics]. Sendability of a composed iterator is unchanged.

**Cleanup riding along**: the internal names still say "pass" (`PassDriver`,
`pass_driver_of`, `for_drivers`' doc; `PassMember` and `pass_elem_ty` went
with B3.3d) and the
dead residue of the deleted `yield fn` origin machinery is still in the tree
(`ORIGIN_PASS_PREFIX` and `pass_or_origin_elem_ty`'s origin branch;
`PassDriver.origin`, the checker's driven-origin check and its
`iter-fn:origin-mutated` message went with D2, 2026-10-30). Rename and delete in one change, cutting by function rather than by
region (the 2026-09-10 lesson).

**Riding along, decided 2026-09-28 (comptime rounds 3 and 7):** two implicits of
**one name at different types** — `fn say_hello<T, S>(it: T | S, ?to_str: (T) ->
Str, ?to_str: (S) -> Str)` — are legal (the "two implicits named `iter` at
different types" this section already anticipates); the **override spelling** is
the typed one section 2c lists (`to_str: (Person) -> Str = f`), with two of the
same name *and* type being one binding [cmp-binder]; what is left to decide is
the **interpolation lowering** — `${it}` with `it: T | S` becomes a `when` over
the union's arms choosing the implicit per arm, which a union value's runtime arm
index permits [type-union-arm-identity] — and whether [interp-to-str] extends to one
`to_str` per arm.

### 17 — Recursive types (DECISION, end of the queue)

Investigated 2026-09-12; nothing needs it, and List-mediated recursion covers its
customers (trees, ASTs, JSON) meanwhile. §0j's sorted containers confirm it: a
tree there is an arena (`List<Node>` with `Int` children), which is the fast
layout on both hosts anyway. Where it stands: nothing rejects a
recursive type, so `struct Node { value: Int, next: Node | None }` passes the
checker, runs on Kotlin and dies at rustc with E0072 — an accept/reject
divergence. Recursion **through `List<T>` already works end to end on both
backends**.

- **Step 1, the diagnostic** (a defect fix, independent of the feature): an SCC
  walk over the type graph (struct fields, union arms, alias expansions;
  `List`/array/fn-typed edges do **not** count — they indirect already) and an
  error at the declaration naming the field that closes the cycle, with the
  `List<T>` encoding as the named remedy. [type-no-cycle] landed the declaration
  refusal for the *direct* case 2026-09-25; the union-arm case is what is left.
- **Step 2, the feature**: a boxing rule for the Rust backend — where the box goes
  (minimal-edge boxing, the presumption), transparency at every use site (literals
  wrap, reads autoderef, matches need explicit derefs since box patterns are not
  stable, partial moves keep working, `Mut` paths get `&mut` via `DerefMut`,
  `copy` deep-clones), and **non-regular (polymorphic) recursion refused in
  Salvo** — a type may recurse only at its own instantiation, or Rust
  monomorphizes forever while Kotlin's erasure accepts it.
- **The semantic edges**, each a small language call: **constructibility**
  (`struct A { a: A }` has no base case — refuse cycles with no optional/union
  escape arm, recommended); **depth, not cycles** (values are acyclic, so
  `to_str`/equality/drop terminate, but each recurses per node and a 100k chain
  overflows generated `toString`/derived `Drop` — accept-and-document for v1);
  **linearity stays out** (refuse recursion + linearity in one declaration for
  v1); and an `iter fn` over a recursive subject snapshots per field, which is a
  deep copy.

### 5 — LSP responsiveness: what is left (user decisions 2026-10-10; phases 1–3 built, 4 recorded, 5 open)

**Superseding design in progress:** the pipeline record (2026-10-20) reorganizes the
compiler into phases with per-file and per-declaration caching and early
cutoff, which would replace phase 4's std-only summary with the general
mechanism. Decided (rounds R1–R5); its §7 is the plan of work.

Phases 1–3 are built (COMPLETED.md, 2026-10-10: [lsp-debounce],
[lsp-parse-cache], [deduce-std-explicit], [std-cache]; the std front shared
across analyses, [lsp-std-front], 2026-10-20). Left of phase 3: a
dependency's own fns are still inferred (they check as project files), so
extending [deduce-std-explicit] to a `std = true` dependency would let the
cache cover it too; and an **unverified concern** (2026-10-20): the cache's
gate does not look at a project module under `core.` that is no std module
(`src/core/x.sv`): `std.core.*` is implicitly visible in every file, std's
included, so such a module could change what std's own names resolve to
while the cache stands in for std's check. Not reproduced.

- **4.** **(Future, recorded.) std as a precompiled library — Option B.** A's larger
   successor: give the front-half a real std-as-a-crate boundary, with std
   checked once into an **owned, index-free** interface summary (contracts
   keyed by module path + item name, not file index) that `resolve`/`check`
   consult, so a project analysis never puts std files in its `Program` at all.
   Persists across compiler runs (keyed by the embedded-std hash) and helps
   multi-project workspaces. A large front-half refactor — it touches the
   "Salvo sees everything in one program" assumption (for std only, which is
   fixed) and needs a path-based parallel to the `FnKey`-keyed tables. Not
   worth it over A until cross-run persistence or workspace scale demands it;
   recorded so A's seam is designed without foreclosing it.
   Its first step (std's modules parsed and expanded once per session) is
   built; std's files are still in every `Program`, and resolution still
   builds their scopes per analysis (~10ms of an edit).

- **5.** **(Follow-on, for `salvo build` — not the LSP.)** Once step 3's annotation
   guarantee holds, std's *IR* is a pure function of std's sources too, which
   is the property the Kotlin std **classes** cache
   (`salvo-backend-kotlin/src/stdlib.rs`, `[kt-std-library]`) relies on today
   by the weaker "verify byte-identical or recompile whole" fallback. Promote
   that cache to a sound IR-keyed std cache, and give the Rust backend an
   equivalent (which §0h already notes needs std's Rust output to stop
   depending on the program first). Step 3 has landed, so this is unblocked.

## Recorded, not scheduled

- **An emitter decisions pass** (user, 2026-10-30: "I expect that there will
  be a sizable pass on emitter decisions, so I'm postponing it"): what the
  D2 merges left wordier or vestigial in the generated code, to be decided
  together rather than one merge at a time. So far: Kotlin spells a lowered
  pick or safe navigation (`?:`, `?.`, `?>`, now `when`s) as a `when`/`if`
  chain where it used to write the native operator; the IR's `Spawn.deps`
  is always empty (a spawn's dependencies are its handler `Construct`'s
  fields since the construction merge) and can go.

- **`x.(F)(a)` for `(F)(x, a)`** (user, 2026-10-28: left out of P19): a
  spelling that calls an arbitrary fn expression with the receiver first,
  e.g. `x.(e.something)()`. Today it is written `(e.something)(x)`.

- **What a language-server edit still costs** (the pipeline record step 8, measured
  2026-10-20, debug, `tools/lsp-timing.py`; ~28–39ms on `examples/cluster`,
  ~40ms on `modules/aws`), largest first:
  1. std's resolution, redone every analysis. Its scopes are the front's
     since 2026-10-30 (by id, viewed: the pipeline record C4); left of it, measured
     the same day on `examples/cluster` (debug): the name binding over std's
     bodies and signatures (`resolve: names`, ~11ms of ~12), the core base a
     project file's scope starts from (built from std's modules per
     analysis), and the address and id maps over std's declarations
     (`resolve: scope ids`, ~3ms with the debug build's check). Sharing the
     names needs `Names<'p>` by id too.
  1b. A program's resolution builds again every scope that took in a module
     comptime changed (gained a clause's members, lost its templates:
     the pipeline record C4 B). Without the std front that is every scope (std's
     core modules gain members), so a one-shot build resolves its scopes
     twice, once for the interfaces and once after stamping (~14 and ~16ms
     on `examples/cluster`, debug). Adding the members to the built scopes
     instead — `add_items` replayed over what each scope took from the
     module, the fn entries put where a build would put them — would build
     them once. Through the front no scope is built twice.
  2. The std seed: `check_once` clones std's whole `Checked` (~5ms) to add the
     project's rows. Needs a layered `Checked` (std's rows read through), or
     a cache that hands its rows out without copying.
  3. `ddb.sv`'s platform boundary (~10ms when walked): `boundary_plan` recurses
     through DynamoDB's nested types for every platform fn, unmemoized (the
     per-file cache hides it unless `ddb.sv` is edited).
  4. A module comptime stamps into is a new module every analysis, so its file
     is always walked (`modules/aws`'s `aws.sv`); a stamped module equal to the
     last one could keep its `Arc`.
  5. The per-file cache's insert clones every cached row (~1.6ms on aws).
- **Per-declaration checker caching** (the pipeline record §7 step 8's second level):
  measured not worth building yet — it saves at most the edited file's own
  walk (~6ms on cluster, ~4ms on aws) and needs rows partitioned by
  declaration inside a file (stamps and route stubs sit past the file's end),
  spans relative to their declaration and stable declaration ids. Revisit when
  single files grow large or the costs above are gone.

- **`lib/std/args`, the features deferred** (from the CLI design record,
  2026-10-20): a `--no-x` form for a defaulted `Bool` (AP-11); `--version`
  (Q17 (c): until asked for; a manifest version or a `Version("1.2")`
  qualifier were the options); options accepted after a sub-command's name
  (`global`, D-3); prefixed flattened groups (`Prefixed("net")`, D-5); a
  valued selector for an exclusive group (`--source url`, D-6);
  provenance-tagged scalar arms (`source: FromUrl Str | FromFile Str` as
  `--from-url V`, §10.3); long-name, metavariable and environment overrides
  (`Long`, `Meta`, `Env` comptime qualifiers, §10.1); config files and shell
  completion (§6); a public combinator API under the derive (Q12 (a));
  declared rules between flags (§5.3); `FieldOf<T>`, a type naming a
  struct's field names (X8).

- **Return-type overloading, the two places it does not reach** (recorded
  2026-10-20 with [fn-overload-return]): effect members still overload by
  parameters only ([effect-member-overload]; `effect_member_index` and the
  member-signature check key on parameters), and `rename fn` names an
  overload by its parameter list, so it cannot single out one of a set that
  differs only in return type. Both would need a spelling decision (a
  `-> R` in the rename's signature, say).

Each was considered and deliberately parked. Nothing here is blocking, and
several are "revisit only if a customer appears".

- **A mixed-element collection constructor that infers a union element**
  (found 2026-10-12, while retiring collection literals). The old literal
  `[1, "a"]` inferred `List<Int | Str>` via bespoke `mk_union` checker logic.
  The replacement constructor `List(1, "a")` goes through the `...rest: T[]`
  variadic, which unifies `T` against each argument and does not widen to a
  union, so it fails overload resolution even with an explicit
  `List<Int | Str>` annotation. Build such a collection element-by-element
  (`add`), or type the first binding to the union. If this proves common,
  options: a variadic that unions its argument types under an explicit union
  element type, or a dedicated multi-type constructor. The variadic
  constructors always had this limitation; only the retired literals papered
  over it.

- **The whole program in the host project** (user, 2026-10-01; ABI D4).
  The alternative to generating only the declarations the platform surface
  reaches: write the entire emitted program into the platform root as
  `*.sv.*`, so the host project is exactly the build's compilation. No
  closure to compute or keep in step with the emitter, at the cost of every
  function body in the root and a much larger checked-in diff per change.

- **Full Salvo expressions in splice holes** (user decision 2026-09-30:
  revisit if splices prove themselves at the small scale). Today a hole
  holds a closed set of emission-dependent spellings with host code only at
  the leaves; the general form would type-check Salvo embedded in host text,
  with typed escapes back to it.

- **Whether the monitor spawn of a plain-effect handler stays** (user
  decision 2026-09-28: keep the split as is for now). `spawn H()` where every
  face of `H` is a plain effect answers an `Addr<E>` and shares one locked
  instance [actor-monitor-handler]; it was the only way to share a stateful plain
  handler between spawns until [effect-handle] made every `use` a handle and
  [actor-spawn-inherit] hands the scope's bindings to a child. What it still buys
  is a plain-effect instance as a **first-class value** (storable, sendable
  in a message, passable in `with`); nothing in std or the examples uses it
  that way. Options if it is revisited: keep as the addr-producing form (two
  spellings for one shape, plus the by-name refusals — `on`, multi-face,
  `mailbox` — that exist to keep it honest); restrict `spawn` to actor
  effects; or restrict it and give `use` a value form, which folds into the
  pending `using` rename in [actor-use-addr]. Decide together with lock-free
  scope-local bindings below, which touches the same shape.

- **Implicit comptime fn resolution** (2026-09-28, comptime round 5, punted): an
  implicit that finds no candidate for a tuple could stamp `std.core.auto`'s comptime fn
  without being asked, making `Mut Set()` over a tuple work bare. Deferred
  because it is the one place a comptime fn would apply without a `by`, which round
  1 decided against for everything else; the remedy `hash by auto, eq by auto`
  at the call (section 2c) is one line, and the missing-identity diagnostic
  should name it.

- **A `for` element of a union-typed list does not fit a parameter of that
  union** (found 2026-09-29 building §4b item 5, not fixed; unrelated to it).
  Repro: `struct A {}` `struct B {}` `type AB = A | B` `fn d(x: AB) [] -> Str
  => x { return "x" }`, then `for c in List<AB>(A {}, B {}) { d(c) }` —
  "no matching overload for `d(proj (A | B))`". The element is a `proj` view of
  the union, and the overload fit does not see through `proj` on a union the
  way it does on a struct. Workaround: bind the element to a typed local
  first. Likely a small fix in the arg-fits rule; needs its own test on both
  backends because the Rust borrow of a union arm is what `proj` renders.

- **Lock-free scope-local bindings** (2026-09-28, the one item left of the
  lock-free pass the one-shape decision deferred; the stateless half landed
  the same afternoon — a stateless handler, the send stub and the mixed façade
  hold no lock, [rs-handle]/[kt-handle]). A *stateful* `use` whose binding no
  spawn, task or dependent handler captures could skip the mutex; this is
  what `use local` used to select by hand. Emission-only, invisible to a
  program. The design, so it need not be re-derived:
  - *The callee never notices.* `draw() [Random]` is `fn draw(random:
    &Random)` and `random.next()` matches on the handle's arm at runtime, so
    a lock-free binding is one more arm (`Local(…)`) chosen at the `use`
    site; `draw` compiles unchanged, and the invariant that a fn's signature
    never depends on its callers holds.
  - *"Nothing captures it" is transitive.* The binding is live across every
    fn reachable from the scope, any of which may spawn a handler inheriting
    `[Random]` or mint a task capturing it. So the property is a call-graph
    fixpoint over the capture sites the checker already records (`constructions`'
    dependencies, a task mint's `effects_passed`) — the shape the deleted
    `handle_requirements`/`call_edges` propagation had.
  - *The handle must stay `Send + Sync`.* Spawns clone handles across
    threads and `Random` is one type for every binding, so a
    `Local(Rc<RefCell<H>>)` arm would make every `Random` `!Send`. The
    options are an `unsafe impl Send` on the local arm's wrapper justified by
    the checker's proof that a `Local` handle never reaches a spawn (a user
    call — unsafe resting on a static analysis), or a lighter single-thread
    lock. On Kotlin none of this arises: skip `__Mon_E`, bind raw.
  - *Why parked*: the saving is an uncontended mutex per call on stateful,
    never-shared bindings; the cost is a whole-program analysis plus either
    an unsafe assertion or a new lock flavour. Build it when a program shows
    that mutex mattering.
- **Eager handle cloning**: `Stamped::new(logger.clone(), clock.clone())`
  bumps two `Arc`s per `use`; fine, noted.
- **The erased-sibling refusal** (`RouteSelector<A>` beside `RouteSelector<B>` in one scope,
  [effect-generic-decl]) was kept through the one-shape change. Its original
  reason — two `__Has_Pick` impls colliding on one fusion struct — is gone,
  and the emitters resolve a call by instance, so lifting it is probably a
  matter of deleting `refuse_erased_sibling` and its test and restoring
  `examples/cluster/`'s `two_ids`/`shop` to inline blocks. Left because the
  two erased handles are still two handlers of one *type* in one scope
  (`__Handle_Pick`), and the cluster example reads fine as two functions;
  try it when the example next changes.

- **Unify the mixed servant with private send members** (2026-09-27). A mixed
  handler's servant emits a handler-keyed `__Msg_H` and calls its members
  directly from `resume`; a private send member of an actor handler now does
  the same through `__Priv_H` [actor-private-send]. They are one mechanism
  in two emitter paths (`emit_mixed_actor_parts` beside `emit_actor_body`);
  re-expressing the servant as "a handler with no actor faces and only private
  members" would delete the first. Churn in two emitters, three goldens and
  `examples/`, for no behaviour — do it when one of the paths next needs a
  change.

- **Value-level parity for hash and random.** Callable `hash` lowers to each
  backend's **native** hashing, so hash *values* diverge across backends —
  accepted deliberately, on the analogy of random numbers. The shape and the
  high-level guarantees are identical (`eq(a,b)` ⇒ `hash(a) == hash(b)` within one
  execution). The possible future reversal, for hash and random *together*:
  language-defined algorithms implemented identically in both runtimes (the
  rejected sketch: FNV-1a 64 over a canonical byte encoding). Until then a program
  must not print or persist a hash value and expect cross-backend identity.
- **`on_idle`'s predicate (DECISION).** The hook fires on the strict quiescence
  condition while the deadlock report fires on a weaker one, so a program stuck
  *with a parked frame* gets the report and exit 1 where the relaxed reading would
  let it react. One line in each runtime either way; the argument for relaxing is
  consistency, the argument against is that `on_idle`'s meaning drifts toward
  "nobody can move", which is the report's job.
- **`size(Str)` outside ASCII (DECISION).** Kotlin lowers it to `String.length`
  (UTF-16 code units), Rust to `chars().count()` (code points), so
  `println("${size("a😀b")}")` prints 3 on Rust and 4 on Kotlin. Needs a decision
  about what a `Str` index *means*, then one lowering per backend. `byte_size` is
  parity-safe by construction and is what the filesystem uses.
- **Intersection types (DECISION)** — whether `Addr<A & B>`-style types join the
  language; recorded 2026-09-17 when the tuple form shipped instead.
- **`const` bindings** — announced (user intent 2026-09-19), not designed. Its
  first customer is recorded: the shareable-handler taxonomy's rung 1 keys on "no
  mutable state", which today means "no fields, no `Mut` constructor parameters";
  `const` immutable fields would join the allowance.
- **Locators through opaque anchors, branded tokens, and the bounds-check
  mitigation ladder** — the group-borrowing ladder's recorded refinements, with
  GhostCell declined on the record (a brand is a scope-bound *lifetime* and actor
  state escapes every scope) and raw pointers / `RefCell` rejected. In
  COMPLETED.md's log for 2026-09-24/25.
- **In-place writes during iteration** — refused by the driven-origins rule
  [iter-fn]. The contents-versus-replacement distinction [deduce-field] is what
  would license it, and the locator model already makes the shape renderable.
- **Field-set inference for [deduce-field]**, and qualifiers on struct fields:
  v1/v2 are written-only, so an unannotated fn keeps the conservative whole-value
  event.
- **`once` inference**, **returning/storing capture-carrying closures**,
  **exactly-once closures consuming a linear capture**, **reassignable borrowed
  locals** (accumulator bodies under a projected return), **same-call borrow/move
  (E0505 shape)**, **the internal qualifier unification**, **L5 field-disjoint
  precision**, **`NotEq` over any `?eq`-capable subject**, **link parameters**
  (with the nested-`proj` source they would make real), and **type-mention tracing
  for instantiation links** — all unforced, each recorded with its shape where its
  rule lives.
- **The linear instantiation ban misses generic effect members** — a hole in what
  shipped: `[linear-generics]` is checked in `resolve_named_call`, and an effect
  member's own generics are not covered, so a linear value can be smuggled
  through one.
- **Nested patterns** (`let ((a, b), c) = …`) are refused in a `let` as in a
  loop, and **struct patterns bind by field name only** (no `..` rest, no nested
  field pattern, no binding of a projection).
- **A generic `List<T>` cannot be interpolated** [interp-to-str] — waits on
  step 5's lift plus `implicit_args` being keyed by something an interpolation
  has.
- **Emission marks everything public**, by decision: [mod-export] is a checker
  rule. Narrowing generated visibility would buy dead-code warnings the suite
  already tolerates and needs the reachability pass to agree.
- **No re-export**, so a facade module declares wrappers; the spelling would be
  `export import a.B`, currently a targeted parse error. And **no example shows
  `export`**, because every program in `examples/` is a single file — a two-file
  example would fix that and would be the tree's first multi-module one.
- **Where dot-notation is normalized** [fn-uniform] — scheduled with §0j step 13
  (the lowering pass is the normalization pass below). The receiver-as-argument-0
  rewrite happens twice and never in the AST, so anything inspecting the *written*
  expression must re-derive the shift (one defect came from exactly that). Two
  shapes when it is picked up: record the normalized argument list per call span
  in a side table (cheap), or a real normalization pass after types are known
  (removes the duplication). It cannot move into `salvo-syntax`: the name must
  resolve, and an `Addr<E>` receiver is not argument 0.
- **Actor-surface prose is owed in `docs/language/`.** Every actor rule is in
  LANGUAGE_SPEC.md, and docs/language/ has the "Where work runs" and "Time"
  chapters — both of which assume vocabulary the document never introduces
  (`actor effect`, `send fn`, `spawn`, `Addr`, `replyto`/`waitfor`, `watch`).
- **Actor leftovers, each with a named trigger**: a self-send into a full own
  mailbox wedges and `k@self` deliberately contributes no deadlock edge
  (**DECISION** when it matters); the deadlock graph is over actor *types*, not
  instances, so a chain of same-protocol workers reads as a self-loop
  (stratification and the timeout form wait for observed false positives —
  writable now that `Timer` exists); the report names actors by index rather than
  by handler and member; a one-thread pool whose occupant sends into a full
  mailbox on that same pool still hangs (only the `main` case is caught); a `use`
  site does not check the `[spawn]` capability; a main-pool task whose answer
  arrives after `main`'s last wait never runs, silently; an effectful discharger
  cannot `drain` a container; `on_idle`'s
  refinements (per-pool firing, naming who is parked, a many-shot form); FC-7 host
  bridging; and the `[waitfor]` spawn-placement diagnostic still states a hazard
  the pump rule removed.
- **Router in-edge leftovers** (found 2026-10-26, with the router in-edges):
  the occupancy report's wording explains a *mixed handler's* façade ("a
  sync member of a mixed handler parks its caller until the servant
  answers") even when the occupied node is a router or a priced shareable
  handler's lock, where what happens is that the caller runs the member on
  its own thread under the lock.
- **Multi-effect and mixed-handler leftovers**: a same-named member across two
  faces still needs `@Effect` at the call even where the parameters distinguish it
  ([effect-at] keys on the name); a dependent multi-face handler is untested;
  overloaded servant members and several faces on a mixed handler are
  first-slice cuts (dependencies were lifted 2026-10-20); one lock behind several faces has no backend
  representation; generic-instance dependencies stay fusion-pinned (the
  representation now exists, so lifting the `handler_handle_deps` exclusion is
  engineering); a `with` item may not have dependencies of its own, and `main`'s
  platform-effect parameters cannot be captured as handles.
- **`local` inference** — the checker writing `local` for you, lifting the
  virality down local-trafficking call chains (the deduction pattern: written
  validates, unwritten infers). **Note (user, 2026-09-26): the meaning of `local`
  is itself being revisited** — a round of questions about `println`'s
  `local Console` dependency was deferred with "I might have misunderstood what
  `local` means", so re-read [use-local] and [effect-local] with the user before
  building anything here.
- **`fn qualifies@Positive`** — migrating qualifier bodies' `fn qualifies` to the
  `@`-scoped shape canonicals used. Note that the shape it would migrate *to*
  changed on 2026-09-26 [fn-attached], so this is now "should a qualifier's
  `qualifies` be declared on its subject type?" and wants re-deciding rather than
  implementing. Handler members stay put: they interact with handler state.
- **Constants: literal establishment and constant subtyping** — the refinement
  round's two remainders. Literal establishment is `listen(8080)` proving itself
  (compile-time evaluation of `qualifies`); constant subtyping is
  `InRange(10, 20)` fitting an `InRange(0, 100)` position, which needs
  per-qualifier semantics for what the constants *mean* — **DECISION**-shaped
  when it is wanted.
- **`enumerate`'s claimed `index` field and a claimed `keys` iterator** — left out of
  claim minting by design: a dependent claim on a struct field names a value the
  struct does not contain, and the map iterator walks a key snapshot. The snapshot
  form (`keys -> List<KeyOf(map) K>`) waits for step 6's variance round.
- **The `Bytes` span twin** needs same-name-different-subject value slots.
- **Binding a view of a temporary** is refused for now (user, 2026-09-11); the
  possible automation is hoisting the temporary into a fresh local, which is what
  the user writes today — not free of judgement, since the temporary then lives to
  the end of the block and a hoist inside a loop changes how often it is built.
- **A runtime file name colliding with an emitted std module** — emission
  refuses when two files claim one path [backend-companion]; the
  workaround is still a rename each time — `hosttime.{rs,kt}`, now
  `throwsignal.kt`. The durable fix, unscheduled: namespace the runtime under
  `salvo_rt/`, which touches every golden, every example and the documented
  `kotlinc`/`rustc` invocations, so it wants its own slice. §0j steps 5, 7 and
  9 move most runtime files into std, which may leave too little to be worth
  it.
- **`to_str(Duration)` stops at seconds**, and there is no `to_str` for `Instant`
  or `Tick`: a wall-clock text form is a date (the calendar layer's), and a
  monotonic reading has no rendering beyond its number. **Cancellation is not in
  the timer surface** (recorded by decision), and **the wall-clock layer is
  designed but unbuilt** — `DateTime` as the calendar view of an `Instant` in a
  zone, `Period`, the two bridges, and a `WallClock` effect whose member must
  **not** be named `now`.
- **The unified test clock fakes `Ticker`; the `Clock` face is untested**, and a
  handler that waits on a *positive* deadline still wedges a `ManualTime` test —
  though it now says so, via the deadlock report. Every clock reading through the
  unified form is a round trip, which is the stance's remaining cost.
  Scheduler-owned virtual time now exists under `test actor` [test-actor];
  whether `ManualTime` and the unified test clock stay, for plain tests, or
  the `time` example moves to an actor test, is open (asked 2026-10-03).
- **`test actor` follow-ups**: services other than the scheduler and the
  timers keep their state across actor tests (the routing table, the stream
  table), which a test using `MemTransport` could notice; `test property`
  will generate the seed. `std.test`'s `trapped_by` and the runtime's
  `guarded` are still two primitives (item 0, point 4).
- **The parked-obligation gap in the deadlock graph**: an actor gated on a token a
  *task* must discharge has a wait-for edge pointing at no effect node.
- **Array elements never narrow**, **deduction inference does not track
  bare-parameter value flow out of branch/loop tails as a move**, **module
  reachability is conservative for non-fn names**, **a private type in an
  exported signature is an opaque type and nothing checks the author meant it**,
  **struct destructuring ignores predicate-qualifier field overrides**,
  **repeating the *same* qualifier in a nested group needs an annotated
  intermediate `let`**, **`Byte` is not operator-numeric**, and **generic
  (`Ty::Var`) operands stay lenient** — each documented where its rule lives.
- **LSP**: `[symbol]` resolution is name-based over the AST rather than
  import-visibility-exact [doc-symbol-ref]; incremental analysis and a cached
  std front-half are now scheduled (section 5); no
  `positionEncoding` negotiation for UTF-8-native clients; signature *hover*
  covers fn decls only (effect members and define fns have no `FnKey`); the
  def-site table is name-keyed, so go-to-definition on a shared effect-member
  name lands on one declaration; and hovering a fate *root* to see what derives
  from it is not built.
- **Fresh-suite speed, remaining steps toward ~10–15s**: the CLI suites (each
  spawns `salvo run`/`compile` and pays its own kotlinc), the rust codegen suite
  (a shared-runtime batch or precompiled `libcore`), and running the batched
  Kotlin programs in one JVM instead of one `kotlin` launch each. Past those the
  floor is the matrix size itself.
