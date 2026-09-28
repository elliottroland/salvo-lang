# EFFECT_FUSION.md — how effects reach code, on both backends

**One shape** (user decisions 2026-09-28, built the same day as ROADMAP §2b —
the record is COMPLETED.md's "One shape for effects" entry): every binding of
an effect is a **handle**, a fn declaring `[A, B]` takes one parameter per
effect, and a dependent handler holds its dependencies as fields. Nothing about
a program decides which of several emissions it gets; there is one, on both
backends, and the two read the same modulo syntax.

This document shows that shape on the program the spec's tests use, then keeps
§4 as the short history of what it replaced and why. Spec rules: [effect-handle]
(the language rule), [rs-handle] and [kt-handle] (the two lowerings),
[effect-handler-deps], [spawn-inherit], [effect-intercept].

## 1. What the checker hands the emitters

Salvo's surface has one idea: a function declares the effects it needs
(`fn work(step: Str) [Logger]`), a handler implements an effect and may itself
depend on others (`handler Stamped [Logger, Clock] of Logger`), and `use`
registers a handler for the rest of a scope. Nothing at a call site names a
handler. The checker resolves, for every call, *which registered instance* each
required effect binds to (`call_effects`, in the callee's `fn_effects` order),
and for every `use` or `spawn`, which instances the handler's dependencies
capture (`use_deps`, `spawn_deps`, with `use_with_items`/`spawn_dep_items`
saying which came from a `with` clause).

The emitters' job is to turn "the instance in scope" into a value the callee
can reach. Both do it with one word:

| word | Rust | Kotlin |
|---|---|---|
| **effect trait / interface** | `pub trait Logger { fn log(&mut self, m: &String); }` | `interface Logger { fun log(m: String) }` |
| **handle** | `__Handle_Logger { inner: Arc<Mutex<dyn Logger + Send>> }`, `impl Logger` by lock-and-forward | the object reference itself; `__Mon_Logger(inner)` (`synchronized`) at every `use` |

## 2. The shape

```
effect Logger { fn log(m: Str) -> None => m }
effect Clock  { fn now() -> Int }

handler PlainLogger [Console] of Logger { fn log(m: Str) => m { println(m) } }
handler TickingClock of Clock { t: Int = 0  fn now() -> Int { t = t + 1; return t } }
handler Stamped [Logger, Clock] of Logger {
    fn log(m: Str) => m { log("[t=${now()}] ${m}") }
}

actor effect Reporter { send fn report(what: Str, done: Reply<Int>) => !what, !done }
handler Reporting() [Logger] of Reporter {
    mailbox { capacity: 4 }
    send fn report(what: Str, done: Reply<Int>) { log("reported ${what}"); done.send(1) }
}

fn work(step: Str) [Logger] -> None => step { log(step) }

fn interception() [Logger, Clock, use] -> None {
    work("plain")
    use Stamped
    work("stamped")
}

fn main() [use, spawn] -> None {
    use StdOutConsole
    use TickingClock()
    use PlainLogger()
    interception()
    let r = spawn Reporting() on pool(1)
    let acked = waitfor done: Reply<Int> { r.report("inherited", done) }
}
```

### 2.1 A fn takes one handle per declared effect

Rust:
```rust
pub fn work(logger: &mut __Handle_Logger, step: &String) { logger.log(step); }
pub fn interception(logger: &mut __Handle_Logger, clock: &mut __Handle_Clock) { … }
```
Kotlin:
```kotlin
fun work(logger: Logger, step: String) { logger.log(step) }
fun interception(logger: Logger, clock: Clock) { … }
```

The parameter order is `Checked::fn_effects`: effects inherited from fn-typed
parameters ([fn-effects]) first, then the written list. The call site passes
its own bindings in the same order (`interception(&mut logger, &mut clock)` /
`interception(logger, clock)`).

### 2.2 A `use` is the handle

Rust:
```rust
let mut console = __Handle_Console::new(StdOutConsole::new());
let mut clock   = __Handle_Clock::new(TickingClock::new());
let mut logger  = __Handle_Logger::new(PlainLogger::new(console.clone()));
```
Kotlin:
```kotlin
val console: Console = __Mon_Console(StdOutConsole())
val clock: Clock     = __Mon_Clock(TickingClock())
val logger: Logger   = __Mon_Logger(PlainLogger(console))
```

Stateless or stateful, every construction goes behind the effect's handle
(one implementation, user decision 2026-09-28; an uncontended lock per member
call). A multi-face handler is one instance and one handle per face
(`let __inst = Arc::new(Mutex::new(H::new(…))); let mut a =
__Handle_A::share(__inst.clone()); …` / `val __h = H(…); val a: A =
__Mon_A(__h); …`), so a stateful multi-face handler is one lock behind several
effect types.

### 2.3 A dependent handler holds its dependencies

Rust:
```rust
pub struct Stamped { __dep_Logger: __Handle_Logger, __dep_Clock: __Handle_Clock }
impl Stamped { pub fn new(__dep_Logger: __Handle_Logger, __dep_Clock: __Handle_Clock) -> Self { … } }
impl Logger for Stamped {
    fn log(&mut self, m: &String) {
        { let __a1 = format!("[t={}] {}", self.__dep_Clock.now(), m); self.__dep_Logger.log(&__a1) };
    }
}
```
Kotlin:
```kotlin
class Stamped(private val __dep_Logger: Logger, private val __dep_Clock: Clock) : Logger {
    override fun log(m: String) { __dep_Logger.log("[t=${__dep_Clock.now()}] $m") }
}
```

One field per declared dependency, in declaration order, whatever the
dependency's kind — plain, an actor effect (the field holds a handle over the
send stub), a generic instance (`__dep_Store: __Handle_Store<i64>` /
`Store<T>`). The `use` clones the scope's handles into `new`, and the
dependency arriving through the enclosing *signature* is no different from
one bound in the same function:

```rust
pub fn interception(logger: &mut __Handle_Logger, clock: &mut __Handle_Clock) {
    work(logger, &("plain".to_string()));
    let mut logger2 = __Handle_Logger::new(Stamped::new(logger.clone(), clock.clone()));
    work(&mut logger2, &("stamped".to_string()));
}
```
```kotlin
fun interception(logger: Logger, clock: Clock) {
    work(logger, "plain")
    val logger2: Logger = __Mon_Logger(Stamped(logger, clock))
    work(logger2, "stamped")
}
```

That is also **interception** ([effect-intercept]): `Stamped` captures the
previous `Logger` handle *before* the new binding is made, which is "binds
strictly outward" in emission. A `with` item is a private instance behind its
own handle: `Stamped::new(logger.clone(), __Handle_Clock::new(FixedClock::new()))`
/ `Stamped(logger, FixedClock())`.

### 2.4 A spawn hands the actor the same struct

```rust
let mut r = ({ let __h = Reporting::new(logger.clone()); let __cap = __h.__mailbox_capacity;
               let __a = salvo_spawn(salvo_pool(1), __cap as usize, Box::new(__Actor_Reporting::new(__h)), __DECODE_Reporting); __a });
```
```kotlin
val r = run { val __h = Reporting(logger); val __a = SalvoSched.spawn(pool(1), __h.__mailboxCapacity, __Actor_Reporting(__h), __Actor_Reporting.__DECODE); __a }
```

The clause's items and the inherited dependencies ([spawn-inherit]) are the
handler's trailing `new` arguments, exactly as at a `use`; the actor body is
`__Actor_H { handler: H }` for every handler, dispatching `E::member(&mut
self.handler, …)`. A monitor spawn answers `__Handle_E::new(H::new(…))` /
`__Mon_E(H(…))`; a mixed spawn answers the handle over the façade.

### 2.5 Fn values, tasks, platform `main`

- **A fn value with effects** (`f: (s: Str) [Logger] -> Str`) is `&mut impl
  FnMut(&mut __Handle_Logger, &String) -> String` / `(Logger, String) ->
  String`; a lambda takes the handle as a typed leading parameter, a named fn
  is adapted (Rust) or passed as `::name` (Kotlin, when it declares exactly
  the type's effects).
- **A task** (free `send fn` minted with `replyto`) takes its inherited effects
  as owned handles and the mint clones the scope's bindings into the closure.
- **`main` with a platform effect** takes `&mut __Handle_E` / the interface;
  the host's `main()` wraps its struct once (`salvo_main(&mut
  __Handle_Telemetry::new(TelemetryHost))`), so a Salvo handler over a platform
  effect captures it like any dependency.
- **`threadsafe platform handler`** changes no emission: the word is the
  declared contract the skeleton prints, which the lock-free stateless pass
  will read.

## 3. What the shape costs, and what it bought

Costs, accepted as decisions (user, 2026-09-28) and recorded for the
optimisation pass in ROADMAP's "Recorded, not scheduled":

- **A lock per member call on every handler**, stateless included — `Mutex`
  on Rust, `synchronized` on Kotlin, uncontended in the common case.
- **A stateful handler captured by two spawns is shared state** (one
  `Arc<Mutex<H>>` cloned twice — Kotlin's behaviour already, since references
  alias). The refusal that used to protect a `use local` binding from capture
  went with `local`.
- **A mixed handler's façade sits behind the handle's lock**, and its sync
  members wait on the servant while holding it: a second caller on another
  thread blocks on the mutex instead of serving its pool. The façade is
  stateless, so the lock-free stateless path removes this.
- **Rust's fn-typed fields are `Arc<dyn Fn + Send + Sync>`** and a handler's
  fn-typed constructor parameter `Box<dyn FnMut + Send>` (they were `Rc` /
  refused), so a handler holding a lambda shares — a Salvo lambda captures by
  value, so the bounds hold for every value the program can build.

Bought:

- **One emission** where Rust had four (plain mode, and three dependent-handler
  shapes — owned handles, the `__Impl_H` fusion form, the `__Prov_H` actor
  provider) chosen by predicates the user could not see; about 1,850 lines of
  the Rust emitter and 450 of the Kotlin one deleted, with `fx.kt`, `__Has_E`,
  `__Fx_N`, `__Prov_…`, `__Deps_H`, `__Impl_H`, `__Hs_…`, `__Mon_E`-over-
  `Box<dyn __Share_E>`, `__Lock_E`, `__Arc_H`/`__Shared_H`.
- **Every feature gap between the shapes closed**: a generic dependent handler
  (`Twice<T> [Store<T>]`) runs on both backends; a `with` clause works on any
  dependent handler; a platform effect arriving through `main` is captured
  like any dependency; an actor member's dependency is a handle it can hand
  to a spawn (no `[rs-handle-bundle]` gap); a stateful multi-face handler
  binds with one `use`; a handler holding a function value shares.
- **The checker lost a layer**: `UseKind`/`use_kinds`, `classify_shareable_use`
  and its blocker list, `handler_handle_deps`, `handle_captures`,
  `handle_requirements`/`call_edges`/`require_handle`, `EffectAvail.local`, the
  call-site "requires a shareable E" rule, the platform-capture, inline-binding
  and signature-capture refusals. What remains is resolution.
- **Readable output**: `fn work(logger: &mut __Handle_Logger, …)` — plain
  mode's shape with a wrapper type.

## 4. History: the shapes this replaced

Until 2026-09-28 the Rust backend had **two emission modes for the same
source**, chosen per program by `program_needs_fusion` (fusion on iff some
reachable handler declared an effect dependency or implemented several
effects), and under fusion **three shapes for a dependent handler**:

- **Plain mode**: one `&mut dyn E` parameter per effect — the shape everyone
  would write by hand. It could not express a handler needing an effect inside
  its members (no room in the trait signature), nor two faces of one handler
  bound to a fn declaring both (`&mut h, &mut h` is E0499). Those two were what
  fusion existed for.
- **Fusion mode** (2026-09-04, reshaped to the Has-accessor design 2026-09-14):
  every fn declaring effects took **one** generic parameter `__fx: &mut __Fx`
  bounded by per-effect accessor traits `__Has_E`, and read each effect by
  UFCS; each `use` built a **fusion struct** `__Fx_main_N { __outer: &mut dyn
  __Prov_…, __h: H }` chained to the previous one, with `__Prov_A_B` conjunction
  traits naming "everything so far". Kotlin mirrored it flatly (`__Fx_N` classes
  with an `override val` per effect, `__Has_E` interfaces in `fx.kt`).
- **Shape A — owned handles** (`handler_handle_deps`, 2026-09-20): a plain-face
  handler whose deps were all bare `[E]` held `__dep_E: __Mon_E` fields, minted
  off "eager handle" variables beside the bindings that were later captured.
  It could be spawned and shared; its cost was that the captured instance was
  a clone-box (`Box<dyn __Share_E>`) — a copy for a stateless handler, an `Arc`
  bump through `__Lock_E<H>` for a stateful one.
- **Shape B — the fusion form** (`__Impl_H` + `__Deps_H`): for a `local E`,
  actor-effect or generic-instance dependency, the member bodies moved into a
  generated trait taking the fused value **per call**; the fusion struct
  implemented the effect by forwarding through a disjoint borrow of `__outer`.
  Exact interception, no clones — but it could not be spawned or shared.
- **Shape C — the actor provider** (`__Prov_H<__D0, …>`): a spawned dependent
  handler owned a flat struct of dependency instances, and the dispatcher built
  the `__Deps_H` view over it per activation, calling shape B's `__Impl_H`.
- **Handle bundles** (`__Hs_…`, [spawn-inherit]'s lift): a capture over a
  signature-supplied effect had only `&mut __Fx` — a borrow, not a handle — so
  the checker recorded a **handle requirement** per fn, propagated it up the
  call graph, and the Rust emitter threaded a hidden bundle parameter. An
  actor's member had neither source, which is why a node group's `init` could
  not call `connect` and `examples/cluster/` spawned its wire actors `with`.
- **The language half**: `use local H()` and `[local E]` (2026-09-20)
  distinguished a scope-local, lock-free, uncapturable binding from the
  shareable default, with a viral call-site rule (`[local E]` down every chain
  that trafficked in a local binding) and a list of handler shapes that
  *required* `use local`.

The survey of 2026-09-27 (this file's previous content) laid out four options:
(1) handles everywhere, (2) fusion everywhere, (3) keep both and close the
gaps, (4) make the A/B choice the `[E]`/`[local E]` distinction in the
language. The user chose (1), extended to *both* backends and to `local`'s
removal: while the language is still being designed, a uniform emission with
fewer edge cases is worth a performance hit, and smarter per-case emissions
are an optimisation pass — one that now has the shape it optimises.
