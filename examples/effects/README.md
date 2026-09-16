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
any of it. Wiring lives at the composition site, not along the call path.

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
  handler's dependencies arrive as one small object it stores
  (`class Stamped(private val __fx: __Fx_2)`), built where the handler is
  registered — `Stamped(__Fx_2(__fx.__fx_Clock, __fx.__fx_Logger))` is the
  outward binding, made of nothing but object references, and the member bodies
  read it rather than rebuilding anything per call.
- **Rust** (`rust/main.rs`): effects become traits, and because a `&mut` cannot
  be in two places, each `use` builds a small generated *fusion* struct that
  owns the new handler and chains to the previous one through `__outer`. A fn
  needing effects takes one generic fused parameter bounded by generated
  accessor traits (`__Has_Logger`), never by the effect traits — which is why
  two effects sharing a member name can never collide there. An intercepting
  fusion carries exactly one `__Has_Logger` impl, its own, while the handler it
  wraps stays reachable through `__outer`: that is section 4's outward binding,
  spelled in borrows.

Both files are generated code, checked in unedited, and they print the same
bytes — `expected.txt`.
