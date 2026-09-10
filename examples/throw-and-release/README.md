# `throw` and releasing what you hold

Leaving a block early with a message, and being right about resources on that
path too. `throw` is the earliest way out there is; `try` is where a throw
lands; linearity is what makes sure a handle held at that moment is not
forgotten. The three belong in one example because the interesting property is
their intersection.

There is no `defer` in this example, and none in the language: it was removed
2026-09-10 (user decision). A block spliced at every exit was the *partial*
answer to "release on every path", and linearity is the full one — it checks the
obligation instead of discharging it out of sight. The cost is visible here:
`close` appears on each path, and the release has to be ordered before anything
that may throw.

Run it:

```bash
cargo run -- run --backend rust   --src examples/throw-and-release/salvo
cargo run -- run --backend kotlin --src examples/throw-and-release/salvo
```

## What it shows, section by section

1. **A resource that cannot be forgotten.** `struct FileHandle : Linear<self>`
   makes every handle carry a use obligation, and the `close` the `Linear`
   group asks for is the only thing that discharges it — `discard` is refused
   on a linear value. `read_size` has two exits and a `close(handle)` on each;
   leave one out and the compiler names the path ("`handle` still owns a linear
   value when it goes out of scope"), so it is a compile error rather than a
   leak.
2. **`throw` keeps the frames in between silent.** `parse_port` declares
   `[Throw<Str>]` and returns `Int`, not an outcome union: `throw` returns
   `Nothing`, so `port_of` forwards nothing and writes no `?`. `strict_port`
   throws two *different* message types, and the thrown arm is their union.
3. **A resource and a throwing call in one function.** `port_from_file` is the
   interaction worth seeing. The code after `parse_port` does not run on the
   throw path, so holding the handle across it is *rejected* — the checker
   forces the release to come first. `copy(handle.name)` is what lets the name
   outlive the handle: a plain binding shares the handle's fate and dies with
   it.
4. **`try` is an intrinsic, and its value is an ordinary union.** There is no
   `Try` effect to declare and no handler to register; `try { … }` evaluates to
   `Ok T | Thrown M`, so `when` reads it exactly like a result. A union message
   is taken apart by binding it at the inner type (`let why: Str | Int = mixed`
   — dropping the `Thrown` claim), and a nested `try` catches only what its own
   body throws: the inner one catches `nope`, then throws again, and the outer
   one catches that.

## What to look for in the generated code

The two backends diverge in *mechanism* and agree on behaviour, which is the
whole design in one file:

- **Rust propagates by value.** `parse_port` is
  `pub fn parse_port(text: &String) -> ControlFlow<String, i32>`, `throw` is
  `return ControlFlow::Break(…)`, and `try` is a **labelled block** —
  `break 'try_1 Union2::U2(__m)` — rather than a closure, so nothing is
  captured.
- **Kotlin propagates by unwinding.** `throw ThrowSignal("not a number: …",
  "Str")` — a stack-trace-less signal carrying the Salvo *type name* of the
  message, because the arm has to be chosen at the `catch`, not at the throw:
  the JVM has no propagation site, and a throwing frame cannot know which `try`
  will catch it. The delimiter dispatches on that tag and rethrows anything
  from outside its set.
- **Propagation is the plain form now.** With nothing pending at the exit, a
  may-throw call forwards with Rust's `?`. Until `defer` was removed, a pending
  deferred block forced the long shape here — a `match` on the `ControlFlow`
  with the release spliced into the `Break` arm.
- **The linear obligation is entirely static.** There is no marker, no flag and
  no drop guard in either output: `close(console, handle)` is just a call the
  checker proved happens on every path.

## Known wart

The nested-union bind (`let why: Str | Int = mixed`) emits an unchecked cast on
Kotlin (`unchecked cast of 'Any?' to 'Union2<String, Int>'`) — the narrowing
makes it unfailable, but `kotlinc` cannot see that. Recorded in COMPLETED.md.
