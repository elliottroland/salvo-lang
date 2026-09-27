# EFFECT_FUSION.md — how effects reach code, on both backends

A survey (2026-09-27) of every way an effect binding travels from a `use` (or
a `spawn`) to the code that performs the effect, with the generated code each
backend emits for it, and a discussion of how the Rust experience could be
unified. Nothing here is decided; the last section is options and trade-offs.
Every snippet below is real output of `salvo compile` at commit `289d89a`,
trimmed of blank lines and comments.

## 1. What the checker hands the emitters

Salvo's surface has one idea: a function declares the effects it needs
(`fn work(n: Int) [Logger, Clock]`), a handler implements an effect and may
itself depend on others (`handler Stamped() [Logger, Clock] of Logger`), and
`use` registers a handler for the rest of a scope. Nothing at a call site
names a handler. The checker resolves, for every call, *which registered
instance* each required effect binds to (`call_effects`), and for every `use`,
*how* the binding is made (`use_kinds`: `Local`, `Bare`, `Monitor`) and which
instances the handler's dependencies capture (`use_deps`, `handle_captures`).

The emitters' job is to turn "the instance in scope" into a value the callee
can reach. Both backends do it with the same vocabulary:

| word | meaning |
|---|---|
| **effect trait / interface** | `Logger` — one method per member, what a handler implements |
| **Has-accessor** | `__Has_Logger` — "I can give you a `Logger`": `fn __get_Logger(&mut self) -> &mut dyn Logger` (Rust), `val __fx_Logger: Logger` (Kotlin) |
| **fused value** | one value implementing every Has-accessor a scope has: `__fx` |
| **provider trait** | `__Prov_Clock_Console` — a trait whose supertraits are a set of Has-accessors, so a `dyn` of it reaches all of them (Rust only) |
| **handle** | `__Mon_Logger` — a clonable, shareable wrapper for an instance, what crosses a seam or is captured |
| **provider struct** | `__Prov_Counting<__D0>` — the dependencies a *spawned* handler holds (Rust only) |

## 2. The two Rust modes

The Rust backend has **two emission modes for the same source**, chosen per
program by `program_needs_fusion`: fusion is on iff some *reachable* handler
declares an effect dependency or implements several effects. Kotlin uses the
identical predicate so the two backends run the same programs through the
same shapes, but its two modes differ far less.

### 2.1 Plain mode: one parameter per effect

```
effect Clock { fn now() -> Int }
handler FixedClock(t: Int) of Clock { fn now() -> Int { return copy(t) } }
fn stamp(n: Int) [Clock, Console] -> Int {
    println("t=${now()}")
    return n * now()
}
fn main() [use] {
    use StdOutConsole()
    use FixedClock(2)
    println("${stamp(5)}")
}
```

Rust:
```rust
pub fn stamp(clock: &mut dyn Clock, console: &mut dyn Console, n: i32) -> i32 {
    println(console, &(format!("t={}", clock.now())));
    return n * clock.now();
}
pub fn main() {
    let mut console = StdOutConsole::new();
    let mut clock = FixedClock::new(2);
    { let __a1 = &(format!("{}", stamp(&mut clock, &mut console, 5))); println(&mut console, __a1) };
}
```
Kotlin:
```kotlin
fun stamp(clock: Clock, console: Console, n: Int): Int { … }
fun main() {
    val console: Console = StdOutConsole()
    val clock: Clock = FixedClock(2)
    println(console, "${stamp(clock, console, 5)}")
}
```

This is the emission everyone would write by hand, and it is what a program
gets as long as no handler in it depends on an effect. It cannot express two
things: a handler that needs an effect *inside* its members (its trait
signature has no room for the parameter), and a handler of two faces bound to
a fn declaring both (`&mut h, &mut h` is E0499). Those two are what fusion
exists for.

### 2.2 Fusion mode: one fused value per scope

Every fn declaring effects takes **one** generic parameter bounded by the
Has-accessors it needs, and reads each effect through UFCS:

```rust
pub fn work<__Fx: __Has_Logger + __Has_Clock>(__fx: &mut __Fx, n: i32) -> i32 {
    __Has_Logger::__get_Logger(&mut *__fx).log(format!("working on {}", n));
    return n + __Has_Clock::__get_Clock(&mut *__fx).now();
}
```

Each `use` builds a **fusion struct** wrapping the new handler and a `&mut
dyn` to the previous fused value (`__outer`), and implements every
Has-accessor the scope now has — forwarding inherited ones through `__outer`,
answering the new one from `__h`:

```rust
pub fn main() {
    let mut __fx  = __Fx_main_1 { __h: StdOutConsole::new() };
    let mut __fx2 = __Fx_main_2 { __outer: &mut __fx,  __h: FixedClock::new(7) };
    let mut __fx3 = __Fx_main_3 { __outer: &mut __fx2, __h: StdLogger::new(…) };
    …
}
pub struct __Fx_main_2<'a, __H> { __outer: &'a mut dyn __Has_Console, __h: __H }
impl<'a, __H> __Has_Console for __Fx_main_2<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console { __Has_Console::__get_Console(&mut *self.__outer) }
}
impl<'a, __H: Clock> __Has_Clock for __Fx_main_2<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock { &mut self.__h }
}
pub trait __Prov_Clock_Console: __Has_Clock + __Has_Console {}
impl<T: __Has_Clock + __Has_Console + ?Sized> __Prov_Clock_Console for T {}
pub struct __Fx_main_3<'a, __H> { __outer: &'a mut dyn __Prov_Clock_Console, __h: __H }
```

The `__outer` field needs a *nameable* type for "everything so far", which is
what the `__Prov_…` traits are: a trait per distinct set of effects, empty,
with the Has-accessors as supertraits and a blanket impl, so `dyn
__Prov_Clock_Console` reaches both through supertrait elaboration. Identical
fusion structs are deduplicated by text within a file.

Kotlin's fusion is the same idea with none of the borrowing:

```kotlin
fun<__Fx> work(__fx: __Fx, n: Int): Int where __Fx : __Has_Logger, __Fx : __Has_Clock {
    __fx.__fx_Logger.log("working on $n")
    return n + __fx.__fx_Clock.now()
}
fun main() {
    val __fx  = __Fx_1(StdOutConsole())
    val __fx2 = __Fx_2(FixedClock(7), __fx.__fx_Console)
    val __fx3 = __Fx_3(__fx2.__fx_Clock, __fx2.__fx_Console, StdLogger(__fx2))
    …
}
class __Fx_2(override val __fx_Clock: Clock, override val __fx_Console: Console) : __Has_Clock, __Has_Console
```

A Kotlin fused class is a **flat** record of references: no `__outer`, no
provider trait, no lifetime. Each `use` copies the previous class's fields
into a new class plus the new handler. Because references alias, the same
`Console` object is held by `__fx`, `__fx2` and `__fx3` at once, which is
exactly the thing Rust's design has to work around.

## 3. How a dependent handler reaches its dependencies

This is where the two backends diverge most, and where Rust has **three**
shapes for one Salvo declaration.

### 3.1 Kotlin: one shape

```kotlin
class Stamped(private val __dep_Logger: __Has_Logger, private val __dep_Clock: __Has_Clock) : Logger {
    override fun log(line: String) { __dep_Logger.__fx_Logger.log("${__dep_Clock.__fx_Clock.now()}: $line") }
}
// at the use site:
val __fx4 = __Fx_3(__fx3.__fx_Clock, __fx3.__fx_Console, Stamped(__fx3, __fx3))
```

A dependent handler is a class whose dependencies are constructor arguments
(each typed by its Has-accessor interface; the current fused value satisfies
all of them, so it is passed for each). Members read `__dep_X.__fx_X`. Spawned
or `use`d, stateless or stateful, shareable or local: the same class. The
`__Mon_E` wrapper (`synchronized`) is added only for a *stateful* shareable
binding.

### 3.2 Rust, shape A — owned handles (`handler_handle_deps`)

When every dependency is a plain, non-generic, non-actor effect declared with
the bare `[E]` (shareable) form, and the handler has no send members and only
plain faces, its dependencies are **captured as handles at construction**:

```rust
#[derive(Clone)]
pub struct Stamped { __dep_Logger: crate::__Mon_Logger, __dep_Clock: crate::__Mon_Clock }
impl Stamped { pub fn new(__dep_Logger: __Mon_Logger, __dep_Clock: __Mon_Clock) -> Self { … } }
impl Logger for Stamped {
    fn log(&mut self, line: String) {
        { let __a1 = format!("{}: {}", __Has_Clock::__get_Clock(&mut self.__dep_Clock).now(), line);
          __Has_Logger::__get_Logger(&mut self.__dep_Logger).log(__a1) };
    }
}
// at the use site: every binding that a later construction captures mints an
// "eager handle", a clonable __Mon_E over a clone of the instance
let mut __bind3 = StdLogger::new(__handle.clone());
let __handle3 = crate::__Mon_Logger::new(Box::new(__bind3.clone()));
let mut __fx3 = __Fx_main_3 { __outer: &mut __fx2, __h: __bind3 };
let mut __bind4 = Stamped::new(__handle3.clone(), __handle2.clone());
```

`__Mon_E` is a clone-boxed `Box<dyn __Share_E>` (the `__Share_E` supertrait
adds `__clone_box`), so the handle is `Clone + Send` whatever handler is
inside. This is the shape that lets a handler be **spawned** (the handle
travels into the actor) and be bound **shareable** without a lock when
stateless; it is also the closest to Kotlin's "the handler holds its
dependency".

Its cost: **the captured instance is a copy**. `Stamped` holds a clone of
`StdLogger`, not the `StdLogger` in `__fx3`. For a stateless handler that is
invisible; for a stateful one bound shareable, the binding is a `__Lock_E`
(an `Arc<Mutex<H>>`) and the clone is an `Arc` bump, so state is shared —
which is why the checker classifies stateful shareable bindings as monitors.
The subtle case is a `use local` stateful handler captured by a spawn: refused
by the checker (a local binding cannot be captured) for exactly this reason.

### 3.3 Rust, shape B — the fusion form (`__Impl_H`)

When a dependency is `local E` (accepts a scope-local binding), an actor
effect, or a generic instance not erasable to a monomorphic type, the handle
form is unavailable and the handler's members move into a generated trait
that takes the fused value **as a parameter at every call**:

```rust
pub struct Stamped {}
pub struct __Deps_Stamped<'a, __P: ?Sized> { pub __p: &'a mut __P }
impl<'a, __P: __Has_Logger + ?Sized> __Has_Logger for __Deps_Stamped<'a, __P> { … forwards … }
impl<'a, __P: __Has_Clock  + ?Sized> __Has_Clock  for __Deps_Stamped<'a, __P> { … forwards … }
pub trait __Impl_Stamped {
    fn log<__Fx: __Has_Logger + __Has_Clock>(&mut self, __fx: &mut __Fx, line: String);
}
impl __Impl_Stamped for Stamped {
    fn log<__Fx: __Has_Logger + __Has_Clock>(&mut self, __fx: &mut __Fx, line: String) {
        { let __a1 = format!("{}: {}", __Has_Clock::__get_Clock(&mut *__fx).now(), line);
          __Has_Logger::__get_Logger(&mut *__fx).log(__a1) };
    }
}
// the fusion struct implements the *effect* itself, forwarding to __Impl_H with a
// view over __outer — the two fields borrowed apart, which is the whole trick
impl<'a, __H: __Impl_Stamped> Logger for __Fx_main_4<'a, __H> {
    fn log(&mut self, line: String) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps_Stamped{ __p: &mut **__outer };
        __Impl_Stamped::log(__h, &mut __deps, line)
    }
}
impl<'a, __H: __Impl_Stamped> __Has_Logger for __Fx_main_4<'a, __H> {
    fn __get_Logger(&mut self) -> &mut dyn Logger { self }
}
```

Here the handler holds **nothing**; the scope's fused value is threaded in.
This is faithful to Salvo's semantics (the dependency is *the* binding in
scope, not a copy) and it is what makes **interception** work exactly: a
handler `of Logger [Logger]` reaches the Logger *outside* it through
`__outer`, so it wraps the previous registration. It cannot be spawned (no
fused value exists on the actor's thread) and cannot be shared (the fusion
struct borrows its parent).

`__Deps_H` exists because Rust trait bounds need a Sized value: a fn generic
over `__Fx: __Has_A + __Has_B` cannot take the `&mut dyn __Prov_…` directly,
so a one-field struct forwards for it. `handler_init` (2026-09-27) runs a
dependent handler's `init` under a `use` with the same three lines, right
after the fusion struct is built.

### 3.4 Rust, shape C — the actor provider (`__Prov_H`)

A **spawned** dependent handler cannot borrow the spawning scope: the actor
outlives the frame. So the spawn hands the actor body a flat struct of owned
handles, one per dependency, and the dispatcher builds the `__Deps_H` view
over it for every activation:

```rust
pub struct __Prov_Counting<__D0> { pub __d0: __D0 }
impl<__D0: Clock> __Has_Clock for __Prov_Counting<__D0> {
    fn __get_Clock(&mut self) -> &mut dyn Clock { &mut self.__d0 }
}
pub struct __Actor_Counting<__D0> { handler: Counting, prov: __Prov_Counting<__D0> }
impl<__D0: Clock> __Actor_Counting<__D0> {
    fn __dispatch(&mut self, msg: __Msg_Counter) {
        let mut __deps = __Deps_Counting{ __p: &mut self.prov };
        match msg {
            __Msg_Counter::Bump(n)     => __Impl_Counting::bump(&mut self.handler, &mut __deps, n),
            __Msg_Counter::Total(out)  => __Impl_Counting::total(&mut self.handler, &mut __deps, out),
        }
    }
}
// spawn site: the handle cloned off the binding's eager handle variable
salvo_spawn(pool, cap, Box::new(__Actor_Counting::new(__h, __Prov_Counting { __d0: __handle.clone() })), __DECODE_Counting)
```

The member bodies are shape B's `__Impl_H` — the same generated trait serves
both a `use` (fusion forwards) and a `spawn` (provider forwards). That reuse
is the one place the three shapes meet.

Kotlin's spawn, for comparison, is the same class with the same constructor:

```kotlin
class Counting<__Fx>(private val __fx: __Fx) : Counter where __Fx : __Has_Clock { … __fx.__fx_Clock.now() … }
val c = run { val __h = Counting(__Fx_3(__fx2.__fx_Clock)); salvo.SalvoSched.spawn(pool, __h.__mailboxCapacity, __Actor_Counting(__h), …) }
```

### 3.5 Handle bundles — dependencies arriving through a signature

Shape A and C need a *clonable handle* at the spawn/use site. When the effect
arrived through the enclosing fn's **signature** rather than a `use` in its
body, the fn only has `&mut __Fx` — a borrow, not a handle. `[spawn-inherit]`
adds a hidden second parameter, the **handle bundle** `__hs:
&__Hs_console__clock`, built by each caller from its own eager handles (or
its own bundle) and threaded up until a frame that has the `use`:

```rust
pub fn interception<__Fx: __Has_Logger + __Has_Clock>(__fx: &mut __Fx, __hs: &__Hs_logger__clock) { … Stamped::new(__hs.logger.clone(), __hs.clock.clone()) … }
// caller:
interception(&mut __fx3, &crate::__Hs_logger__clock { logger: __handle3.clone(), clock: __handle2.clone() })
```

An **actor's member** has neither source: its dependencies are in the
provider (shape C), which the dispatcher exposes only as the `__Deps_H`
view — a borrow. That is why a node group's `init` cannot call `connect(me)`
(which spawns two `[Transport]` actors) today, and why `Booting` in
`examples/cluster/` spawns them `with MemTransport(…)` — a `with` clause
constructs a private instance right there, which needs no handle from scope.
Kotlin has no such gap: `__fx` is a reference and passing it is cloning it.

### 3.6 Fn-typed values, tasks, monitors

- **A fn value with effects** (`f: (n: Int) [Clock] -> Int`) is emitted as
  `&mut impl FnMut(&mut dyn __Has_Clock, i32) -> i32`; the call site wraps
  a named fn in a closure that rebuilds a fused value from the `dyn`
  (`__FxDyn_main_3 { __outer: __prov }`), since the callee is generic over
  a Sized `__Fx`. Kotlin passes `(Clock, Int) -> Int` and wraps with
  `__Fx_3(__fx0)`.
- **A task** (free `send fn` minted with `replyto`) captures its effects as
  handles in the closure — the same eager-handle variables.
- **A monitor** (stateful shareable `use`, or a plain-effect `spawn`) is
  `__Lock_E<H>` (`Arc<Mutex<H>>`) boxed into `__Mon_E`; a `threadsafe
  platform handler` is `__Arc_H` (`Arc<H>` with `&self` members, no lock).
  Kotlin: `__Mon_E(inner)` with `synchronized`, or the raw instance.

## 4. The unification question

Salvo has one concept; Rust has three emissions of it (A/B/C) plus plain
mode, chosen by predicates the user cannot see (`handler_handle_deps`,
`program_needs_fusion`, `use_kinds`), each with its own generated types
(`__Mon_E`, `__Share_E`, `__Lock_E`, `__Arc_H`, `__Fx_N`, `__FxDyn_N`,
`__Prov_…` traits, `__Prov_H` structs, `__Deps_H`, `__Impl_H`, `__Hs_…`).
The cost is not correctness — every shape is tested and the two backends
agree — but *legibility of the output*, *feature gaps between shapes*
(shape B cannot spawn; A cannot intercept exactly; an actor member cannot
mint handles), and *emitter complexity* (the fusion code is ~3,000 lines of
`emit.rs`).

The root cause is one fact: Rust has no aliasing `&mut`. Kotlin's fused class
holds references to every binding and hands the same references to every
handler; Rust must choose, per instance, between **one owner and borrows**
(shapes B, fusion structs, `__outer`) and **shared ownership** (`Arc`, shapes
A and C). The three shapes are that choice made three different ways for
three situations.

### Option 1 — handles everywhere (make shape A the only shape)

Every binding becomes a `__Mon_E` handle at its `use`; every dependent handler
holds handles; every fn declaring `[E]` takes `&mut __Mon_E` per effect (or a
struct of them). Plain mode disappears; so do `__Fx_N`, `__outer`, `__Prov_…`
traits, `__Deps_H`, `__Impl_H`, handle bundles.

- Stateless handlers: a clone per capture, free.
- Stateful handlers: **every** stateful `use` becomes an `Arc<Mutex<H>>`,
  including `use local` — a lock on every member call of a `Mut List`
  wrapped in a handler, on a single thread. Rust's uncontended mutex is
  ~20ns; still, it is the thing the whole "shareable by default, `local`
  opts out" design was built to avoid paying blindly.
- Interception: a handler `of Logger [Logger]` holds a handle to the
  *previous* Logger — a clone of a stateless one, an `Arc` to a stateful
  one. Semantically right; the `__outer` chain becomes a chain of handles.
- Spawns and actor members: trivially uniform — a handle is a handle
  anywhere, so `connect` from `init` just works and handle bundles vanish
  (an effect through a signature *is* a handle).
- Output: `fn work(logger: &mut __Mon_Logger, clock: &mut __Mon_Clock, n: i32)`
  — plain mode's shape with wrapper types. Very readable.

Trade-off in one line: uniformity and readability for a lock on every
stateful non-shareable handler, and a `Box<dyn>` indirection on every call.
Could be softened with a `__Local_E<H>` handle (a `Rc<RefCell<H>>`) chosen
by `use_kinds == Local`, which keeps the one-handle-type-per-binding story
while paying a runtime borrow flag instead of a mutex — the classic Rust
"RefCell where you'd have used &mut" trade.

### Option 2 — fusion everywhere, dependencies always threaded (shape B only)

Every dependent handler is `__Impl_H` taking `__fx` per call; a spawn gets a
provider (shape C is already B behind a provider). Shape A and its eager
handles go; `__Mon_E` survives only as the seam type for `Addr`/monitors.

- Exact semantics everywhere (the dependency is the binding, never a copy),
  interception exact, no clones.
- Spawn still needs owned handles for the provider, so handle bundles stay,
  and the actor-member gap (3.5) stays unless the provider learns to hand
  out clones — which it can, since a provider's fields are the handles.
- Output for handlers gets *worse*: every member of every dependent handler
  is `fn m<__Fx: __Has_A + …>(&mut self, __fx: &mut __Fx, …)`, and a handler
  cannot be used as a plain Rust value implementing its effect trait —
  only the fusion struct implements `Logger`.
- Removes a mode and ~a third of the fusion code; keeps the hardest third.

### Option 3 — keep both, make the seam explicit and the gaps closed

Keep A for "shareable, plain deps" and B for "local/actor/generic deps", but:

- Make the **provider hand out handles**: `__Prov_H` already holds
  `__Mon_E`s; add `fn __handle_E(&self) -> __Mon_E` to `__Deps_H` so an
  actor member can build a handle bundle. Closes 3.5's gap (`init` calling
  `connect`) without changing any shape. ~half a day.
- Rename to say what things are: `__Mon_E` → `__Handle_E` (it is not a
  monitor; `__Lock_E` is), `__Prov_…` traits → `__All_…`, `__Deps_H` →
  `__View_H`. Zero semantic change, large legibility gain.
- Emit the mode choice as a comment on each handler (`// bound by handle:
  every dependency is a shareable plain effect`), since the predicate is
  invisible in the source.

Trade-off: cheapest, keeps three shapes, closes the one functional gap.

### Option 4 — move the choice into the language

`[E]` vs `[local E]` already distinguishes "shareable, may be captured" from
"call-only". That *is* the A/B distinction: a handler whose dependencies are
all `[E]` can hold handles; one with a `[local E]` dependency must be
threaded. So the language already says which shape a handler gets — the
emitter's `handler_handle_deps` is (almost) that rule plus two exclusions
(actor deps, generic instances). Making it exactly that rule, and stating it
in [rs-effect-fusion] as "a `[local E]` dependency is threaded, a `[E]`
dependency is held", would let a reader predict the emission from the
signature. The exclusions become work items: actor-effect dependencies as
handles (an `Addr` *is* a handle; the missing piece is `__Has_E` for an
`Addr<E>`'s stub), and erased-generic instances (already done for
`Pick<E>` — 2026-09-27's `effect_only_args`).

This is Option 3's cleanup plus a rule the spec can state. It doesn't reduce
the shapes, but it makes them *derivable*, which for a language whose Rust
backend is meant to be read is most of the value.

### What I would do

Option 3's provider-handles change now (it unblocks the `init`→`connect`
case and is small), Option 4's rule-and-rename as the next step (no runtime
change, spec gets a sentence a reader can hold), and hold Option 1 in
reserve as the "one shape" endpoint if the two-shape story still reads as
two languages after that. Option 2 I'd drop: it trades the readable shape
(A) for the unreadable one (B) and keeps the hard problems.

## 5. Where the code is

| piece | Rust `emit.rs` | Kotlin `emit.rs` |
|---|---|---|
| mode predicate | `program_needs_fusion` (1071) | same name |
| handle-dep predicate | `salvo_core::handler_handle_deps` (shared) | shared |
| fusion struct per `use` | `emit_fusion_inner` (~10620) | `emit_use` builds `__Fx_N(...)` |
| `__Impl_H` + `__Deps_H` | `emit_dependent_members`, `emit_deps_adapter` (~4843) | none (class ctor args) |
| forwarding impl on the fusion | `emit_forward_impl` (~4919) | none |
| actor provider | `emit_actor_body` (~4274), `spawn_provider` | carrier type param `<__Fx>` |
| handles / monitors / locks | `emit_monitor_stub` (~3164), `emit_lock_adapter` | `emit_monitor_stub` |
| handle bundles | `handle_bundle_param`, `handle_bundle_arg` (~5273–5330) | not needed |
| fn-value fusion (`__FxDyn`) | search `__FxDyn` | closure wrapping `__Fx_N(__fx0)` |

Spec rules: [rs-effect-fusion], [rs-monitor], [rs-handle-bundle],
[effect-handler-deps], [use-local], [spawn-inherit], [kt-effect-fusion].
