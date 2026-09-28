# effects

An effect is a capability: a set of functions that only code holding a
*handler* for them may call. A function declares what it needs in `[...]`, and
`use` registers a handler for the rest of the scope. This program runs seven
effects' worth of that idea together, because the interesting part is not one
effect — it is what happens when several are in play and one of them wraps
another.

Run it:

```bash
cargo run -- run --backend rust   --src examples/effects/salvo
cargo run -- run --backend kotlin --src examples/effects/salvo
```

## What to look for

**1 — an effect, a handler, and `use`.** The effect declares what can be
called; the handler implements it and may keep private **state** for its
lifetime. `TickingClock` returns a reading five ticks later each time, which is
how the program stays deterministic without a real clock.

**2 — several effects in one signature.** `stamp` declares `[Clock, Console]`
and may do exactly those two things. Its callee `banner` declares `[Console]`
alone: a caller's set covering a callee's is the whole rule, and nothing is
passed at the call site.

**3 — a handler that depends on another effect.** An effect *member* may not
declare effects of its own — the interface would then differ per
implementation — so a `Logger` that needs to print declares the console the way
a function does, as an effect list on the handler, and the compiler supplies it
where the handler is registered. Two things follow: the `use` site writes
`use PlainLogger` and never mentions the console, and callers of `log` declare
`[Logger]` only. Swap in a logger that needs a *different* effect and no
signature in the program changes. The dependencies have no names because
nothing could refer to one: a handler body reaches an effect by calling its
members, like all Salvo code.

**4 — interception.** A handler may depend on **the effect it implements**. The
dependency binds *outward*, to whatever was registered before it, so the
handler wraps that one instead of replacing it — and the `log` call inside
`Stamped.log` goes one layer out rather than recursing. `Stamped` depends on
two effects at once, one of them its own. Interceptors stack: with `Numbered`
registered over `Stamped`, a line is numbered and then stamped, and the output
shows both prefixes in that order.

Notice *where* the dependencies surface. `interception` is the function that
registers `Stamped`, so `interception` is what needs a `Clock` in scope; the
function actually calling `log` declares `[Logger]` and knows nothing about
any of it. Wiring lives at the composition site, not along the call path: a `use`
captures its dependencies as handles, and those handles may come from the
enclosing *signature* as well as from a binding in the same function, so
wiring code can receive the effects it wires.

**5 — shadowing is not wrapping.** A `use` for an effect already in scope takes
over for the rest of the block, and what it shadowed comes back at the closing
brace. `QuietLogger` has no dependency and swallows what it is given, so the
line inside the block disappears from the output and the next one is logged
again — the plain shadowing case, with no interception involved.

**6 — two effects that share a member name.** `record` on both `Audit` and
`Metrics` is the natural spelling. A bare call resolves through whichever
effect actually has a handler available; where both do, `record@Audit(what)`
picks — the same `@` selector that picks a module's overload.

**7 — two instances of one generic effect.** `Setting<Int>` and `Setting<Str>`
are two separate capabilities and can be in scope together. A call picks its
instance from the expected type (`let retries: Int = setting()`) or from a
written type argument (`setting<Str>()`).

Its handler shows a second thing worth reading: a handler **keeps** its
constructor argument for its whole life, so a member cannot hand the stored
value out — every call would be moving the same one [effect-state-store]. The
remedy is `copy`, and for a *generic* value the handler is the wrong party to
ask how to copy it, so `?copy` arrives as an implicit parameter filled at the
call site [copy-implicit]. Written without it, the Rust backend clones and the
Kotlin backend refuses outright: neither is a bug, they are the two ways a
missing copy shows up.

## The composition root

`main` is the only function in the program that names a handler; everything
else names capabilities. Reading `main` top to bottom is reading the entire
configuration — and a handler registered there is reached by every function
below it that declares the effect, without being threaded through the calls in
between.

## In the generated code

Worth a look, because effects are the feature whose lowering is least obvious:

- **Kotlin** (`kotlin/main.kt`): effects become interfaces, handlers classes. A
  dependent handler holds its dependencies as **fields typed by the effect's
  interface** — `class Stamped(private val __dep_Logger: Logger, private val
  __dep_Clock: Clock)` — passed where the handler is registered, and every
  `use` wraps the instance in the effect's `synchronized` monitor
  (`val logger: Logger = __Mon_Logger(Stamped(logger, clock))`). On the JVM a
  reference is a handle, so that is the whole mechanism; the monitor is what
  makes a stateful handler safe to hand to a spawn.
- **Rust** (`rust/main.rs`): the same shape with a real handle type. Every
  effect gets a `__Handle_E` — an `Arc<Mutex<dyn E + Send>>` implementing the
  effect's trait by lock-and-forward — and every binding *is* one: `let mut
  logger2 = __Handle_Logger::new(Stamped::new(logger.clone(), clock.clone()))`.
  A fn declaring `[Logger, Clock]` takes `&mut __Handle_Logger, &mut
  __Handle_Clock`, a dependent handler holds `__dep_Logger: __Handle_Logger`
  fields, and interception is a handler capturing the *previous* handle
  before it is bound — section 4's outward binding, spelled as a clone. (Until
  2026-09-28 Rust built a generated *fusion* struct per `use` and threaded one
  fused value; the one shape replaced it.)

Both files are generated code, checked in unedited, and they print the same
bytes — `expected.txt`.
