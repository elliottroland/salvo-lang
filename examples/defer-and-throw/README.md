# `defer` and `throw`

Leaving a block, on every path. `defer` runs code on the way out; `throw` is the
earliest way out there is; `try` is where a throw lands. The three belong in one
example because the interesting property is their intersection: a resource that
must be released *including* when a function leaves early with a message.

Run it:

```bash
cargo run -- run --backend rust   --src examples/defer-and-throw/salvo
cargo run -- run --backend kotlin --src examples/defer-and-throw/salvo
```

## What it shows, section by section

1. **`defer` is block-scoped and spliced at every exit.** Two in one block run
   in reverse declaration order (`lifo`), and one in a loop body belongs to
   *that* body, so it runs once per iteration — including the iteration that
   `continue`s and the one that `break`s (`per_iteration`).
2. **A resource that cannot be forgotten.** `struct FileHandle : Linear<self>`
   makes every handle carry a use obligation, and the `close` the `Linear`
   group asks for is the only thing that discharges it — `discard` is refused
   on a linear value. `read_size` has two exits and one `defer { close(handle) }`
   covering both; forgetting a path is a compile error, not a leak.
3. **`throw` keeps the frames in between silent.** `parse_port` declares
   `[Throw<Str>]` and returns `Int`, not an outcome union: `throw` returns
   `Nothing`, so `port_of` forwards nothing and writes no `?`. `port_from_file`
   is the interaction worth seeing — a live linear handle across a throwing
   call, released by the `defer` on the throw path too. `strict_port` throws two
   *different* message types, and the thrown arm is their union.
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
- **`defer` is spliced on Rust and `finally` on Kotlin.** Look at
  `port_from_file` in `rust/main.rs`: the release is rendered *twice*, once on
  the throw path inside the `match` and once at the ordinary exit. Kotlin nests
  `try { … } finally { … }`, which gives LIFO for free.
- **The linear obligation is entirely static.** There is no marker, no flag and
  no drop guard in either output: `close(console, handle)` is just a call the
  checker proved happens on every path.

## Known wart

The nested-union bind (`let why: Str | Int = mixed`) emits an unchecked cast on
Kotlin (`unchecked cast of 'Any?' to 'Union2<String, Int>'`) — the narrowing
makes it unfailable, but `kotlinc` cannot see that. Recorded in PROGRESS.md.
