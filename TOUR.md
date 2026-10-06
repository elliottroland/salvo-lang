# A tour of the compiler

Written 2026-10-06 for the design conversation about ROADMAP §0j steps 12–13
(the ownership tables and the lowering pass). It shows the code as it stands
now, piece by piece, with snippets, so you can follow without opening the
files. Line numbers drift; names and `[rule-labels]` do not, so `grep` for
those. At the end (§9–§10) is what the IR work would change and what I need
you to decide.

Contents

1. One program, start to finish
2. The crates
3. `salvo-core`: a map
4. The `Checked` tables: the checker's output
5. Following the sample program through the tables
6. `salvo-backend`: the shared layer
7. The Rust emitter
8. The Kotlin emitter
9. What is a Salvo fact and what is a backend's: the classification
10. What the IR work would be

---

## 1. One program, start to finish

Every example below comes from this program (`tmp/tour/main.sv`, compiled with
both backends; it runs and prints the same thing on both).

```
struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int => b: Mut {
    b.n = b.n + 1
    return b.n
}

fn label(s: Str, n: Int) -> Str => s, n {
    return "${s}/${n}"
}

fn consume(items: List<Int>) -> Int => !items {
    return size(items)
}

iter fn countdown(from: Int) -> Emitted Int | Finished {
    state {
        at: Int = from
    }
    if at <= 0 { return finished() }
    at = at - 1
    return emitted(at + 1)
}

fn main() [use] {
    use StdOutConsole()
    let b = Mut Box { n: 1, tag: "t" }
    println("1. ${b.n} ${bumped(b)}")
    println("2. ${label(b.tag, bumped(b))}")
    let xs = list_of(3, 2, 1)
    let twin = copy(xs)
    println("3. ${consume(xs)} ${size(twin)}")
    for x in countdown(3) {
        println("4. ${x}")
    }
}
```

The pipeline, in the order the code runs it:

```
 .sv text
    │  salvo-syntax: lexer → parser → desugar (iter fn → struct + minter + next)
    ▼
 Module ASTs ──► Program (source.rs, program.rs): all files, flat symbol tables
    │  salvo-core: expand (comptime stamping, by-auto), resolve (per-file scopes)
    ▼
 check_program ──► Checked        ← the whole output of the front half:
    │                               about 100 side tables, mostly keyed by (file, span)
    │  salvo-core: erase (effect-only generics), reach (which modules)
    ▼
 salvo-backend::driver            ← check, erase, resolve, reach, in one place
    ▼
 the emitter (Rust or Kotlin) walks each reached module's AST, consulting
 Checked at every node, and prints target source
```

The two decisive facts about the design:

* **The AST is never rewritten after checking.** The checker records what it
  learned in `Checked`, keyed by `(file index, Span)`; an emitter walks the
  original AST and looks each node up by its span.
* **Both emitters are walkers that *decide* things as they go.** Some
  decisions come out of `Checked` (what a call resolved to). Others the
  emitter makes itself from local state (whether a name is a borrow in the
  Rust output). The IR conversation is about the second kind.

## 2. The crates

| crate | lines | what it is |
|---|---|---|
| `salvo-syntax` | ~15k | lexer, parser (6.7k), `desugar.rs` (iter fn, etc.), AST, `visit`/`visit_mut`, spans, diagnostics. No dependencies. |
| `salvo-core` | ~42k | everything between the parse and the emitters. `check.rs` alone is 30k. |
| `salvo-backend` | ~1.4k | what the emitters share: `driver.rs` (check/erase/resolve/reach), `emit_util.rs` (walkers, `EmittedFile`, ABI stamping, literal desugaring, `checker_refs`). |
| `salvo-backend-rust` | ~19k | `emit.rs` (17k), `imports.rs`, `intrinsics.rs`, runtime `.rs` files. |
| `salvo-backend-kotlin` | ~10.5k | `emit.rs` (9.7k), `imports.rs`, `intrinsics.rs`, `stdlib.rs`. |
| `salvo-cli` | | the `salvo` binary; also the LSP (`lsp.rs`, `analysis.rs`). |
| `salvo-test`, `salvo-testkit` | | the `salvo test` runner; toolchain probing and the test cache. |

`std/` (the standard library, in Salvo) is embedded into the `salvo` binary at
build time, which is why a `std` edit needs a `cargo build`. Most of what used
to be runtime code is now Salvo there (the scheduler, collections, the stream
table); the emitters keep only `intrinsic` lowerings and a few host files.

## 3. `salvo-core`: a map

By what you would come looking for.

**Getting from text to a `Program`**

* `source.rs`: discovering files, module paths from directory layout, test
  annexes (`foo.test.sv`), std shadowing.
* `program.rs`: `Program` (all files and their ASTs) and `Symbols` (flat
  global tables: `fns`, `structs`, `effects`, `handlers`, `type_aliases`,
  `intrinsic_types`, `effect_of_fn`, …). Names that several modules declare
  get a *key* (`Node§main`); `typekey.rs` is that scheme [type-identity].
* `expand.rs`, `comptime.rs`: the pre-resolution expansions. `comptime.rs`
  stamps a `comptime fn` at a concrete type and unrolls `[for]`/`[when]`/`[if]`
  into an ordinary fn, giving every stamped node a fresh synthetic span
  (`fresh_span`) so the span-keyed tables still work.
* `resolve.rs`: per-file scopes (`ModuleScope`): what names a file sees.
* `erase.rs`: an effect-only generic (`ActorGroup<E>`) is erased for the
  backends.

**The checker and its vocabulary**

* `check.rs` (30k lines): type checking, overload resolution, narrowing, flow,
  effects, actors, qualifiers. It fills `Checked`. Start with
  `check_program`, then `check_call`, `check_effect_call`, `resolve_named_call`.
* `types.rs`: `Ty` (the checker's type: `Named`, `Qualified`, `Union`,
  `Tuple`, `Fn{contract}`, `Lit`, …), subtyping, overload ranking.
* `deduce.rs`: the `=> !p` / `=> p: Mut` clause; infers which parameters a
  call consumes (`ParamDeduction.kept`). `lends.rs`: which parameters a result
  holds a borrow of (`proj`). `refine.rs`: qualifier refinements. `place.rs`:
  places (`h.field`), the keys of flow analysis. `literal.rs`: how literal
  types are represented in a union. `wire.rs`: what may cross a machine.

**Tables derived *from* `Checked` for the backends**

These are the files I have been adding. Each is a question an emitter used to
answer for itself.

| file | question it answers |
|---|---|
| `reach.rs` | which modules does the program use? which fn does each call resolve to (`resolved_fn_keys`)? |
| `features.rs` | does the program need the scheduler / the wire codecs? |
| `param_mode.rs` | is a parameter moved in, lent, or lent mutably? |
| `mut_lends.rs` | which fns hand back a result a `Mut` position reads? which cover `canbe` parameters? |
| `borrows.rs` | does this call's result borrow from its arguments? which union arms? |
| `copyplan.rs` | what does `copy(x)` duplicate, for a value of this type? |
| `naming.rs`, `effects.rs` | the emitted name of an overloaded fn or effect member |
| `abi.rs`, `platform.rs` | what a host project must see; platform handlers/fns/types |

**Project-level**: `manifest.rs` (`salvo.toml`), `lock.rs` (`salvo.lock`
protocol hashes), `deadlock.rs` (actor cycle check), `route.rs`, `prereq.rs`.

## 4. The `Checked` tables: the checker's output

`Checked` is a struct of maps and sets, almost all keyed by
`Key = (usize, Span)`: *file index, source span of the node*. A few kinds, with
examples of what they say about the sample program.

**What is this expression's type?**

```rust
pub expr_ty: HashMap<Key, Ty>,       // after flow narrowing
pub repr_ty: HashMap<Key, Ty>,       // the type it is *stored* as, when it differs
pub coercions: ...                   // wrap a value into a union arm, etc.
```

`repr_ty` is the narrowing record. In `if value is Int { … value … }` the
checker says `value` here has `expr_ty` `Int` but `repr_ty` `Box | Int`; the
emitter sees the difference and reads through the union's arm.

**What did this name resolve to?**

```rust
pub call_fn: HashMap<Key, FnKey>,      // call span → the fn declaration
pub fn_refs: HashMap<Key, FnKey>,      // a fn used as a value
pub implicit_args: HashMap<Key, Vec<ImplicitArg>>,   // what fills ?cmp, ?hash…
pub interp_to_str: HashMap<Key, FnKey>,               // the to_str behind "${x}"
pub member_calls: HashMap<Key, MemberCall>,  // effect-member calls (new)
pub dot_calls: HashSet<Key>,                 // `x.f(a)` read as `f(x, a)` (new)
```

For `println("1. ${b.n} ${bumped(b)}")`: `call_fn` has an entry for the
`bumped(b)` call; `interp_to_str` has one for each `${…}` part (both are
scalars here, so they point at the intrinsic `to_str(Int)`); `member_calls` has
an entry for `println` (it is a member of the `Console` effect):

```rust
pub struct MemberCall { pub effect: String /* symbol key */, pub index: usize }
```

**What does the flow analysis say about this read?**

```rust
pub linear_moves: HashSet<Key>,      // this read consumes a linear value
pub state_takes: HashSet<Key>,       // this read takes a handler's state field
pub moved_projections: HashSet<Key>, // `s.field` moved out of a consumed root
pub binding_modes: HashSet<Key>,     // this `let` is a move-mode bind event
pub deductions: HashMap<FnKey, Vec<ParamDeduction>>,  // per fn, per parameter
```

For `consume(xs)` the fn's `deductions` say `items: kept = false` (the clause
`=> !items`), and in `main` the checker treats `xs` as consumed afterwards.
That is why `twin = copy(xs)` is written before `consume(xs)`.

**Lending (views of other values)**

```rust
pub derived_calls: HashMap<Key, …>,   // calls whose result is a view of an argument
pub lending_calls: HashMap<Key, Vec<usize>>,
pub mut_lend_calls: HashSet<Key>,     // …and a `Mut` position reads it
pub handle_muts, virtual_place_binds, distinct_pairs, covered_calls …
```

**Rule of thumb for finding one:** grep `pub .*: HashMap<Key` in
`check.rs` near the top of `Checked`; every field has a doc comment with the
rule label that introduced it.

A limit worth knowing: the tables hold **one value per span**. Comptime
stamping (fresh spans per node) exists partly to keep that true. Any future
pass that *invents* nodes must register table entries for them.

## 5. Following the sample program through the tables

Here is each interesting line of `main`, with what the checker recorded and what
each emitter made of it.

### 5.1 `let b = Mut Box { n: 1, tag: "t" }`

Rust: `let mut b = Box { n: 1, tag: "t".to_string() };`
Kotlin: `val b = Box(n = 1, tag = "t")`

Every Rust local is `let mut` (Salvo's mutability is not locally decidable;
`unused_mut` is allowed in generated code).

### 5.2 Parameter modes: `bumped`, `label`, `consume`

```
fn bumped(b: Mut Box) -> Int => b: Mut        // kept, Mut
fn label(s: Str, n: Int) -> Str => s, n        // kept, kept
fn consume(items: List<Int>) -> Int => !items  // consumed
```

Core decides (`param_mode.rs`):

```rust
pub enum PassMode { Moved, Lent, LentMut }

// top-level fn: the deductions, then the declared type
pub fn fn_param(&self, key: Option<FnKey>, param: &Param) -> PassMode {
    if param.variadic || is_fn_group(&param.ty) { return PassMode::Moved; }
    if matches!(param.ty, Type::Fn { .. }) {
        return if key.is_some_and(|k| self.owns_callbacks(k)) { Moved } else { Lent };
    }
    let kept = key.and_then(|k| self.checked.deductions.get(&k))
        .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
        .map(|d| d.kept).unwrap_or(true);
    if !kept { return PassMode::Moved; }
    kept_mode(&param.ty)          // LentMut if the type (or its elements) carry Mut
}
```

The Rust backend spells it (`emit.rs`):

```rust
impl ParamMode {
    fn spell(mode: PassMode, ty: &Type) -> ParamMode {
        match (mode, matches!(ty, Type::Fn { .. })) {
            (PassMode::Moved, _) => ParamMode::Owned,
            (PassMode::Lent, true) | (PassMode::LentMut, _) => ParamMode::RefMut,
            (PassMode::Lent, false) => ParamMode::Ref,
        }
    }
}
// and the scalar exception, in Emitter::param_mode:
if self.modes().is_copy(&param.ty) { return ParamMode::Owned; }
```

Result, Rust:

```rust
pub fn bumped(b: &mut Box) -> i32 { … }
pub fn label(s: &String, n: i32) -> String { … }   // n: scalar, by value
pub fn consume(items: Vec<i32>) -> i32 { … }       // moved
```

Kotlin ignores the modes: `fun bumped(b: Box)`, `fun consume(items: List<Int>)`.
The same `ParamMode` is consulted again at every **call site**
(`emit_args_for_params_of`): a `Lent` parameter gets `&arg`, a `LentMut` gets
`&mut arg`, a `Moved` one gets the value.

### 5.3 The hoist: `println("1. ${b.n} ${bumped(b)}")`

The language orders the two parts left to right: `b.n` is read before `bumped`
changes it. Kotlin runs that as written:

```kotlin
println(console, "1. ${b.n} ${bumped(b)}")
```

Rust's `format!` borrows every argument until it returns, so `b.n` (a borrow of
`b`) collides with `bumped(&mut b)` (E0502). The Rust emitter hoists the read:

```rust
println(&console, &({ let __r1 = b.n; format!("1. {} {}", __r1, bumped(&mut b)) }));
```

Where that lives: `plan_read_hoists` (and `plan_call_hoists`,
`plan_interp_hoists`) in `emit.rs`. It gets, for a list of sibling expressions,
which one *holds a borrow* and which one *is the `&mut`*:

```rust
// a call's arguments: from the callee's parameter modes
let holds:    Vec<bool> = all.iter().zip(&modes)
    .map(|(a, m)| is_place_expr(a) && matches!(m, Some(Ref) | Some(RefMut))).collect();
let mut_here: Vec<bool> = modes.iter().map(|m| matches!(m, Some(RefMut))).collect();
self.plan_read_hoists(&all, &holds, &mut_here)   // → the `let` lines
```

and records each hoisted sibling in `hoisted_reads: HashMap<Key, String>`
(span → `"__r1"`), which the rendering paths consult so the hoisted read
appears as the local. The same machinery handles line 2 (`label(b.tag,
bumped(b))`, where the read is a borrowed argument and the hoist is a clone).

A different gap, fixed this session: an interpolation part that reads a place
*through a call* (`${get(d, i)} ${replace(d, i, 0)}`) is not a place, so the
plan misses it. `emit_string` now formats each part to text in turn when a
later part takes a `&mut` and no place hoist applies ([rs-interp-sequence]).

The *fact* here is Salvo's (left-to-right evaluation, and which arguments are
lent mutably). The *remedy* (a `let`) is Rust's.

### 5.4 `copy(xs)` and the read of `xs`

Rust: `let mut twin = xs.clone();`. Kotlin: `val twin = xs`.

Rust's `clone` is always deep, so it needs no plan. Kotlin shares structure, so
it needs to know which parts to duplicate. Core says (`copyplan.rs`):

```rust
pub enum CopyPlan {
    Identity,                                   // nothing can mutate it: share
    StrBuilder,                                 // a `Mut Str`
    Platform(String),                           // a host type's own `copy`
    Elements { deque: bool, mutable: bool, elem: Box<CopyPlan> },
    Struct { name: String, fields: Vec<(String, CopyPlan)> },
    Array,
    Recur { name: String, mutable: bool },      // a struct that holds itself
}
pub fn copy_plan(symbols: &Symbols<'_>, ty: &Ty) -> Option<CopyPlan>
```

`List<Int>` is immutable all the way down, so Kotlin's plan is `Identity` and the
copy is the same object. For `List<Mut Node>` it would be `Elements` over a
`Recur`, and Kotlin would call a generated `__copyMut_Node`.

### 5.5 Reading `xs` at `consume(xs)`

Rust: `consume(xs)`: moved, no clone. The checker consumed `xs` (deduction),
`Moved` mode says the argument is handed over, and an owned local in a moving
position is just moved.

How does Rust know a *read* of a borrowed name in an owning position must be
cloned, while an owned one need not? From **emitter state**, not from a table:

```rust
enum BindKind { Owned, Ref, OptRef, RefMut, SelfField, ElemMut }   // per name
enum ValueMode { Read, Own }                  // what the current position wants
```

`self.bindings: HashMap<String, BindKind>` is filled at about 37 sites as the
emitter passes binders (parameters, `let`, `is`, `for`, lambda parameters);
`self.mode` is set by about ten callers (a `let` and a `return` ask for `Own`;
an interpolation or a `&T` argument stay in `Read`). The core of the decision,
`emit_owned_inner` for an identifier:

```rust
match self.bindings.get(id.name.as_str()) {
    Some(BindKind::Ref) | Some(BindKind::RefMut) if !copy => format!("{place}.clone()"),
    Some(BindKind::Ref) | Some(BindKind::RefMut)          => format!("*{place}"),
    …
    _ => place,                                     // an owned local: move it
}
```

There is no last-use analysis anywhere. "Owned local in a moving position" is a
move because the checker already rejected any later use.

### 5.6 The effect call and the dot call: `println`, `b.tag`

`println(...)` is a member of `Console`. Both emitters used to decide that by
asking `effect_of_fn` ("is this name a member of some effect?"), which cannot
see scopes. Now they ask the checker's answer:

```rust
if let Some(call) = self.checked.member_calls.get(&(self.file_idx, span)).cloned() {
    let effect: &str = self.symbols.effects.get_key_value(call.effect.as_str()) …;
    let member = self.symbols.effects.get(effect).and_then(|e| e.fns.get(call.index));
    // handler expression comes from `effect_calls` (the instance), args from the
    // member's parameter modes, …
}
```

Rust renders it as `println(&console, …)` (the handle is an explicit argument),
Kotlin as `println(console, …)`.

### 5.7 `for x in countdown(3)`: the iterator

`iter fn` is a **desugaring in the syntax crate**: there is no state machine
and nothing suspends. The body is the `next`. The parser rewrites the fn into
three ordinary items before the checker sees anything:

```rust
// Rust output
pub struct __Iter_countdown_Int { pub from: i32, pub at: i32 }
pub fn countdown(from: i32) -> __Iter_countdown_Int {
    return __Iter_countdown_Int { from: from, at: from };     // the "minter"
}
pub fn next(__p: &mut __Iter_countdown_Int) -> Union2<i32, Finished> {   // your body
    if __p.at <= 0 { return Union2::<i32, Finished>::U2(finished()); }
    __p.at = i32::wrapping_sub(__p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(i32::wrapping_add(__p.at, 1)));
}
```

The `for` loop is the only generated control flow, and it is a plain loop that
calls `next` (the checker records the driver in `for_drivers`):

```rust
let mut __loop1_pass = countdown(3);
while let Union2::U1(mut x) = next(&mut __loop1_pass) {
    println(&console, &(format!("4. {}", x)));
}
```

```kotlin
var __loop1_pass = countdown(3)
while (true) {
    val __loop1_step = next(__loop1_pass)
    if (__loop1_step !is Union2.U1<Int, Finished>) { break }
    val x = __loop1_step.value
    …
}
```

The two are the same shape rendered differently: Rust's `while let` pattern,
Kotlin's `is` test and `.value`.

## 6. `salvo-backend`: the shared layer

`driver.rs` is the front half every emitter starts with:

```rust
pub fn check_for_emission(program) -> Result<(Program, Erased, Checked, warnings), _>
pub fn resolve_for_emission(program, &mut checked) -> (Symbols, Resolution)
pub fn reach(program, &resolution, &symbols, &checked, abi) -> Reach
pub struct Reach {
    reachable,      // modules the program uses [mod-used-only]
    closure,        // ABI mode: the kept declarations
    platform_effects, abi_full,
    abi_modules,    // ABI mode: the host-facing modules
    emitted,        // every module to write: one rule for both backends
}
```

`emit_util.rs` is a bag of shared helpers (23 walkers at the last count):
collecting mutated names, finding `is` bindings, rebuilding collection literals
as constructor calls (`literal_as_call`), the ABI item names, and
`checker_refs`: the set of names a module refers to (its source, the fns the
checker resolved, the type names in `expr_ty`), which both backends use to
write their imports.

Two small mechanisms the emitters register with:

* `imports::note(name)`: an emitter calls it where it *synthesizes* a name the
  checker never saw (`__Actor_H`, a union struct, `..._qualifies`), so the
  module's import plan includes it.
* `salvo_core::features::module_features`: does this module need the
  scheduler or the wire codecs? The emitters OR the answers over all emitted
  modules.

## 7. The Rust emitter

`emit_program_mode` (top of `emit.rs`) is the driver. In order: check, reach,
choose the emitted modules, then for each module `Emitter::new(…)` and
`emit_module`, then imports, union files, the runtime files that are needed.

`Emitter` is one big struct (about 90 fields). The groups that matter here:

```rust
checked, symbols, program, file_idx      // read-only inputs
bindings: HashMap<String, BindKind>       // the ownership state, see §5.5
mode: ValueMode                           // what the current position wants
hoisted_reads: HashMap<Key, String>       // span → local a read was hoisted into
mut_lend_fns, mut_call_sites, mut_forward_sites   // from core's `mut_lends`
covered_fns                               // from core's `mut_lends`
elem_places, virtual_places               // mutable element handles
borrowed_arm_locals                       // locals holding a view with `proj` arms
mutated: HashSet<String>                  // names assigned in this fn
```

The shape of expression emission is a set of functions that pass a
*wanted mode*:

* `emit_expr(e)`: the owned rendering (a value). Sets `mode = Own`.
* `emit_read(e)`: the borrowing rendering. Sets `mode = Read`.
* `borrowed_arg(e)`, `borrowed_mut_arg(e)`: an argument for a `&T` / `&mut T`
  parameter.
* `emit_let`: the richest statement. It chooses between a virtual place, a mutable
  element handle, a real borrow (`let x = &place`, when the value is a pure
  place and `x` is never reassigned), an optional borrow (`Option<&T>`), and
  the ordinary owned local with a clone. All of that is in ~250 lines around
  the `bindings.insert` calls.

Narrowing has its own sub-machinery: `narrowing_of`, `narrow_unwrap`,
`narrowed_borrow`, `owned_optional_local`, `ident_unwrap`. They decide
whether a narrowed read is `x.u2()`, `*x.u2()`, `x.u2().clone()` or
`x.as_ref().unwrap()`, from `repr_ty`, the binding kind, Copy-ness and the mode.

What is in the Rust emitter purely because of rustc:

* `ElemMut` / virtual places (a live `&mut` would be E0502 against later reads).
* The hoists.
* `ty_is_concrete` (a generic `T` might be `Copy` after monomorphization).
* `&&T` avoidance (a `clone` of a reference).
* The `__loc` twins (a lent mutable result is a locator index, re-applied to
  the container at each use).

## 8. The Kotlin emitter

Similar skeleton, far less state. Kotlin has no ownership to render:

* parameters are passed by reference; `PassMode` is ignored;
* `copy` renders core's plan (`render_copy`), and generates `__copy_Name` for a
  struct that holds itself;
* the throw signal, byte buffers and ordered-struct comparison are Kotlin's own
  runtime pieces (`needs_throw`, `needs_bytes`, `needs_compare`);
* value-position `if` is native; a block in value position is `run { … }`; a
  `loop` with `break value` uses a result variable.

What the two emitters share that could be shared more: rendering unions
(`UnionN` generator), `try`/throw, boundary checks, struct/effect/platform
interface rendering. Each has its own code for these today.

## 9. What is a Salvo fact and what is a backend's: the classification

You set the rule: core holds concepts Salvo knows, backends interpret them.
Applying it to every decision that is still in an emitter:

| decision | Salvo fact | backend concern | where it is now |
|---|---|---|---|
| parameter moved / lent / lent mutably | ✔ (`PassMode`) | by value, `&T`, `&mut T`; scalars; fn values | core + Rust spell |
| what `copy` duplicates | ✔ (`CopyPlan`) | the spelling | core + Kotlin render |
| call → declaration, member, dot call | ✔ (tables) | the syntax of the call | core |
| this read consumes / takes / moves a projection | ✔ (`linear_moves`, `state_takes`, `moved_projections`, `binding_modes`) | `mem::take`, move, `.unwrap()` | core tables |
| a call's result borrows from its arguments (`proj`) | ✔ (`derived_calls`, `borrows.rs`) | `Option<&T>`, `Union2<&T, …>` | core + Rust |
| this fn's result is read by a `Mut` position | ✔ (`mut_lends`) | locator twins | core + Rust |
| **a `let` binds a borrow instead of a clone** | ✘ | an optimization: Salvo says a read is a copy | Rust `emit_let` |
| **the kind of a local (`BindKind`)** | ✘ | what Rust type the local has | Rust `bindings` |
| **a read is moved, cloned, dereferenced or borrowed** | partly | which of those a *position* needs | Rust `ValueMode` + `BindKind` |
| **hoists** | the ordering constraint ✔ | the `let` ✘ | Rust |
| value-position block / `loop` | ✔ meaning | `run {}`, result variable, native | each backend |

The rows in bold are what I was going to build tables for in step 12. Looking at
them through your rule, **I did not build them**:

* `BindKind` and the borrow-`let` are Rust's own state about its own output
  types. A core table of them would just be Rust's logic moved into core.
* The per-read kind is a function of three things: the *position* (what the
  consumer wants), the *root* (owned or borrowed in the output), and the type.
  The position is Salvo-level (it is the callee's `PassMode`, or "a `let` wants
  a value"), and so is what the checker already knows about consuming. The
  root is Rust's.

What *would* be a Salvo fact is **the demand on each expression**: for every
expression the walk could say "this value is moved into a consumer / lent to one
/ lent mutably / dropped / returned". Everything the Rust emitter derives from
`ValueMode` is a function of that. It is the one new core table step 12 could
still justify; §10 is where it fits.

## 10. What the IR work would be

Three shapes were on the table (side tables, an AST-to-AST pass, a new IR).
We agreed on tables now and an AST-to-AST pass for the rewrites, working
toward a real IR. Concretely:

**The demand table (if we build it).** One walk in core, in evaluation order,
that records for each expression its demand. Sketch:

```rust
pub enum Demand { Moved, Lent, LentMut, Discarded, Returned, Stored }
pub demand: HashMap<Key, Demand>;        // in Checked, or a sibling struct
```

Inputs it needs are all in core already: `PassMode` for arguments, the
statement kind for `let`/`return`/field/store, the checker's consume tables.
What it replaces: the ten sites that set `ValueMode`. What stays Rust's:
`BindKind` (to know whether a given read of a given name clones).

**The hoist pass (AST-to-AST).** Instead of `plan_read_hoists` deciding
textually while printing, a pass produces a rewritten expression:

```
before:  println("1. ${b.n} ${bumped(b)}")
after:   { let __r1 = b.n;  println("1. ${__r1} ${bumped(b)}") }
```

as a `Block` whose first statement is a `Let` with a temporary name. The Rust
emitter prints it with its ordinary block/let code. Kotlin, if it did not call
the pass, is unaffected. Two things it has to do that side tables did not:

* register `expr_ty` (and coercion/`repr_ty`) entries for the nodes it creates,
  since the emitter will look them up by span, as comptime stamping does;
* run after the demand table, because what counts as a borrow is a function of
  the modes.

The pass would live in `salvo-backend` (an opt-in helper), not in core, because
*hoisting* is a remedy for one kind of target.

**Value-position control flow.** The same shape: `let v = if c { a } else { b }`
is native in both targets, so nothing to do; `let v = loop { … break x … }` has
no native form in Kotlin and a result variable is the rewrite. A helper in
`salvo-backend` takes a predicate saying what the target supports natively.
(Core does not know the target. The backend passes its own capability set to the
helper it chooses to call.)

**The IR itself.** If the above works, a lowered body IR is "the output of
these helpers, given a name": a tree with explicit `Move`/`Clone`/`Borrow`/
`Take` and `Temp` nodes that an emitter only prints. That is the step that would
let a third backend be short. It is not this round.

### Questions for the IR discussion

1. **The demand table:** build it (a small Salvo-level walk, Rust the only
   consumer), or leave `ValueMode` in Rust as its own state? My reading of your
   rule is "build it, if its only inputs are Salvo facts", but the risk is a
   table with one consumer.
2. **Where do the helpers live?** `salvo-backend` (shared, opt-in) is my
   proposal. The alternative is a fourth crate for lowering only, if the helpers
   grow large.
3. **Synthetic nodes.** The AST-to-AST pass registers table entries for what it
   creates. Do you want that done by copying the replaced node's entries (cheap,
   byte-identical) or by giving the IR its own type records (clean, bigger)?
4. **Byte-identical, or changing?** You said unify now for behaviours. For the
   hoist pass I would keep the Rust output identical while moving the logic, then
   change output in separate commits. Confirm.
5. **Order.** Demand table → hoist pass → `fn` variants (the `__loc` twins are a
   rewrite too) → value-position helpers → consider the IR.

A note on language: nothing here needs a language-design decision. The one
place where the language might change is if we wanted `let x = place` to say
whether it borrows (today it is always "a copy, and the compiler may elide it"),
and I am not proposing that.
