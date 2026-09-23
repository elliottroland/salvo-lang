# Salvo

Salvo is an experimental high-level programming language with C-like syntax
that transpiles to **Kotlin** and **Rust**. It supports the
transition from the JVM to a non-garbage-collected language by letting one
codebase target both worlds — with no explicit pointers, references,
ownership, or borrowing in the source.

**Salvo** is an acronym for the values the language is designed to support:

- **S**implicity — a small, imperative core; minimal control flow.
- **A**ffordancy — the type system exposes what you can *do* with data,
  via qualifiers like `Ok Int` or `Mut List<T>`.
- **L**ocality — no hidden state: effects make dependencies explicit and
  handlers are registered in scope with `use`.
- **V**erifiability — strong static types, exhaustive `when` over unions,
  checked effects, and deductions that let backends compile to safe Rust.
- **O**rthogonality — features compose instead of overlapping: structs hold
  data, functions define behavior, qualifiers refine types, effects carry
  capabilities.

The compiler is written in Rust. See [LANGUAGE.md](LANGUAGE.md) for the full
specification, [examples/](examples/) for programs that compile and run today —
each checked in with the Rust and Kotlin the compiler generated for it and the
output it prints — [ROADMAP.md](ROADMAP.md) for what is still to come, and
[COMPLETED.md](COMPLETED.md) for what is built and why.

## Building

```bash
cargo build        # build the compiler
cargo test         # run the test suite

# Compile and run in one step (requires the backend's toolchain on PATH):
cargo run -- run --backend kotlin --src ./my_project
cargo run -- run --backend rust --main ./my_project/main.sv

# `--main` names the file holding `main`, which is how you pick between
# several entry points; on its own it also implies its own directory as the
# source directory. Pass both to start at a file in a subdirectory:
cargo run -- run --backend rust --src ./my_project --main ./my_project/bin/tool.sv

# Output goes to `.salvo_tmp_run` and is cleaned up afterwards; `--target DIR`
# and `--clean-target before` change where and whether.

# Type-check a directory of .sv sources without generating code:
cargo run -- analyze --src ./my_project              # or --format json

# Run the tests a source tree declares (`test "name" { ... }` blocks in
# `<module>.test.sv` files beside the modules they test):
cargo run -- test --src ./my_project                 # or --list, or a filter
cargo run -- test --src std                          # the standard library's own

# Generate the host implementation skeleton for every `platform effect`
# into ./my_project/platform/ (written once, never overwritten):
cargo run -- platform generate --backend kotlin --src ./my_project

# Start a language server (LSP over stdio) for editor integration:
cargo run -- lsp
# A VS Code extension bundling syntax highlighting and the language server
# lives in vscode/ — see vscode/README.md.

# Or generate the target sources and build them yourself:
cargo run -- compile --backend kotlin --src ./my_project --target ./out
kotlinc out/*.kt out/core/*.kt -d classes && kotlin -cp classes salvo.main.MainKt

cargo run -- compile --backend rust --src ./my_project --target ./out_rs
rustc --edition 2021 out_rs/main.rs -o program && ./program
```

## A taste of Salvo

```
struct Person {
    name: Str,
    surname: Str? = None,   // Str? is shorthand for Str | None
    age: Int
}

// `Ok`/`Err` and their constructors come from `core.result`; declaring
// your own pair of tags takes the same four lines.
fn check_age(person: Person) -> Ok Int | Err Str {
    if person.age >= 0 {
        return ok(person.age)
    }
    return err("negative age")
}

fn describe(person: Person) [Console] {
    let result = check_age(person)
    when result {
        is Ok {
            println("${person.name} is ${result}")     // result: Ok Int here
        }
        is Err {
            println("Error: ${result}")                // result: Err Str here
        }
    }
}

fn main() [use] {
    use StdOutConsole()
    describe(Person {name: "Roland", age: 36})
}
```

## Features at a glance

- **Data**: structs (defaults, spread `...`, destructuring), tuples
  (destructuring and positional reads, `t.0`), immutable strings
  with `${}` interpolation — and `Mut Str` for building one, which reaches
  the whole immutable surface by dropping its `Mut`. `Bytes` is the same pair
  for binary data: a buffer you read, `Mut Bytes` to build one, `to_hex` and a
  strict UTF-8 bridge — not a `List<Byte>`, so an octet costs an octet.
- **Collections**: `List`, `Set`, `Map` and their sorted counterparts, each
  with a literal — `[1, 2, 3]`, `{"a", "b"}`, `{"k": "v"}`, and `Mut` in
  front for a mutable one. `Set` and `Map` iterate in **insertion order on
  every backend**, so a program's output does not depend on the target it
  was compiled for. Comparison is a **capability**, not a built-in: `a == b` is
  `eq(a, b)` and `a < b` is `cmp(a, b) < 0`, so equality is opt-in and a type
  joins in by declaring the function — `: auto Hashed<self>` generates `hash`
  and `eq`, `: auto Ordered<self>` a `cmp` too, checked where they are
  declared. A structure that *stays* ordered names the ordering it holds as a
  type argument (`Heap<T, ?cmp: (T, T) -> Int>`), so a heap built under one
  ordering is a different type from one built under another and the two refuse
  to mix — and a claim does the same, so a `Sorted` list is searched by the
  ordering that sorted it rather than by the host's. Arrays stay for fixed-size data and the variadic boundary.
- **Unions & nullability**: `A | B` types, `T?` as `T | None` (no null
  value), flow-sensitive narrowing via `is` — of variables and of field
  chains (`p.address.city`) — and exhaustive `when`.
- **Qualifiers**: type-level annotations (`Ok T`, `Surname Person`) enabling
  overloading, union tagging, and precise checks; `Mut` opts structs and
  types (`canbe Mut`) into mutability. `is` narrows a value to a more
  specific type, `^` widens it by removing a qualifier (`when o { is ^Ok { … } }`
  reads the union inside an `Ok` claim). A qualifier is a claim about a
  value's *contents* (`NonEmpty`) or about where the *handle* came from
  (`provenance qualifier Authenticated of Request`) — only the former can
  be invalidated by mutation. A qualifier can also state what functions it
  does *not* own do to its claim (`refn add(list: Mut List<T>, elem: T)
  => list: +NonEmpty`), which is how a mutating call keeps a property it
  has never heard of. Structs and qualifiers can be namespaced
  under a struct (`Environment.Id`), giving wrapper types without nesting.
- **Everything is an expression**: `if`/`when` produce values; branch types
  union together. `when` is always exhaustive — over a union's arms with a
  subject, or as a condition chain with a mandatory `else` when written
  without one (`when { n < 0 { … } else { … } }`), which is how a chain of
  conditions produces a value that is never absent. Conditions are `Bool`;
  there is no truthiness.
- **Functions**: overloading by argument types — the most specific *scope*
  wins first (`core`, then imports, then your module, then the function's own
  scope), then the most specific signature; an ambiguity is an error, and the
  caller picks with `size@core.list(xs)` or `rename fn size2 = size(...)` —
  dot-notation
  (`list.size()` ≡ `size(list)`), variadics, lambdas and generics.
  Iteration is ordinary Salvo: a **pass** is a struct with a `next`, `for` is
  sugar for calling it until `Finished`, an `iter fn` writes the pass struct for
  you, and `map`/`filter`/`reduce` take a pass — `params Yield<It, T>` is a
  bundle of implicit parameters, not a trait, so a type of your own joins in by
  declaring one function.
- **Algebraic effects**: effects declare capabilities, handlers implement
  them, `use` registers handlers in scope — dependencies are always visible
  in signatures. A handler may itself depend on an effect (declared as an
  effect list on the handler, like a function's); the compiler supplies it
  from the enclosing scope, so callers never mention it. That effect may be
  **the one the handler implements**, which is interception: the
  dependency binds strictly outward, so a policy handler wraps the one
  already registered, and a later `use` shadows an earlier one. One handler may
  implement **several effects** (`of Timer, TimerCtl`): one state, one face per
  effect, which is how a public protocol and an administrative one share an
  implementation — and a `spawn` of one hands back an addr per face, so who holds
  which face decides what they may do. A function
  *value* that performs an effect declares it in its type
  (`(s: Str) [Logger] -> Str`), and the effect is supplied by whoever calls
  the value — so a higher-order function inherits its callback's effects
  and needs no annotation of its own.
- **Actors**: an **actor** is an effect handler bound asynchronously — `spawn`
  instead of `use`. An `actor effect` declares the protocol (`send fn` members,
  which enqueue and answer nothing), a handler of it is ordinary Salvo, and
  `spawn Counting() on pool(2)` gives it a mailbox — whose depth the handler
  declares, `mailbox { capacity: 8 }` — and answers an
  `Addr<Counter>`; the `on` clause is optional, and omitted means the pool the
  spawn itself runs on, which in `main` is a pool `main` is the single worker
  of. Its state is its own and its members run one at a time, so
  the serialization *is* the mutual exclusion. An answer travels back through a
  **linear** one-shot `Reply<T>`: minted with `replyto`, which parks a
  continuation on one of your own members so no thread waits anywhere, and
  discharged exactly once because the compiler says so — and a **free
  `send fn`** is the same continuation without an actor behind it: work that
  runs by being scheduled, so an ordinary synchronous function can wire future
  work with `replyto` and return, on the pool it is running on unless an `on`
  clause says otherwise. Where a frame does mean
  to wait, `waitfor` is the bridge — no capability, no declaration: occupancy
  is inferred, priced by the deadlock graph, and reported by name at runtime.
  `thread()` answers a `Dedicated Pool` the `on` clause consumes, for work
  that *wants* a thread of its own. A wait serves its own pool while it waits rather than merely
  blocking on it. Actors depend on each
  other through ordinary effect lists (`use addr` binds one to a scope, so
  callers never learn their capability is an actor), hold queues of obligations
  in state, `watch` each other die, and a topology whose actors could wait for
  one another is reported *before* it runs — tasks included, since a task's
  sends count against whoever minted it. A fault nobody was watching reaches
  the pool's sink (`pool(n, sink)`), or is named on stderr. The scheduler is a library in each
  backend's runtime — no runtime baked into your code, and identical behaviour
  on both.
- **Time**: `Duration` for a span, `Instant` for a wall-clock point and `Tick`
  for a monotonic one — kept apart so a deadline cannot be measured against a
  clock that NTP can step. Reading either clock is a capability (`Clock`,
  `Ticker`), sleeping is an `actor effect` (`Timer.after(wait, done)`) whose
  fire arrives as an ordinary message, and `ManualTime` is a pure-Salvo fake
  wearing two faces — so a test that would wait two seconds advances virtual
  time instead and always prints the same thing. The posture the module is built
  around is to **pass time rather than read it**: a fire carries the tick it came
  due at, and a function handed its times declares no effect and needs no fake at
  all. Where a reading must agree with a deadline, a six-line test clock over the
  timer makes them one virtual clock. Imported rather than implicit:
  `import time` brings the whole module.
- **Non-resumption**: a function that may leave early declares
  `[Throw<Str>]` and keeps its own return type; `throw(message)` returns
  `Never`, so intermediate frames stay silent. The delimiter is
  `try { ... }`, whose value is `Ok T | Thrown M` — an ordinary union, so
  `when` reads it like any result.
- **Deductions**: a clause after the return type — `-> T => list: Mut` —
  describing what a function does to its parameters and what its result
  borrows of them; whatever it leaves unsaid is inferred from the body. The
  ownership contract for the Rust backend, and the one place Salvo states a
  borrow: `proj[from: list] T` returns an element without copying it, a
  struct with `proj` fields is a view, and a copy happens only where the
  program writes `copy`.
- **Files**: `std`'s filesystem is the whole language in one surface — an
  `Fs` effect whose members cover paths *and* streams (so a double fakes all
  of it), linear `InStream`/`OutStream` tokens that must be closed, a linear
  `FsError` that cannot be dropped in silence (`ignore` it, or `detach` its
  kind to keep it), a `Lines` pass for `for line in p`, one-shots
  (`read_to_str`, `read_lines`, `write_str`, `copy_file`) for the common case,
  bytes as themselves (`read_bytes`/`write_bytes` over `Bytes`, sharing one
  stream and one position with the text reads), a fill-a-buffer read for the
  loop that cannot afford a payload per step (`read_to`, `read_line_to`,
  and a `chunks` pass), and exact byte offsets —
  `write` answers its byte count, `position` reports one, and
  `open_read_at(path, offset)` reopens at one, since streams stay
  forward-only. The machine's filesystem is a `platform handler` at the
  bottom, so swapping it swaps the world the program runs in — for
  `RestrictedFs(root)`, which scopes it to one directory, or `MemFs`, which
  runs the same code with no disk at all.
- **Interop**: a `platform effect` declares what the program needs from its
  target language, and a `platform handler` is a host implementation of an
  *ordinary* Salvo effect — registered with `use` like any handler, so the
  entry point stays put. The compiler generates the interface and
  `salvo platform generate` writes the host implementation skeleton into
  `platform/`, so the *target's* compiler checks the two against each other.
  Those two are the whole interop surface: std's own primitives are
  `intrinsic`, lowered by code inside each backend, and `intrinsic` is the
  compiler's to declare.
- **Modules**: a file is a module, and its declarations are **private to it
  unless they say `export`** — so a module's public surface is exactly what it
  writes down, and the standard library's own plumbing is unreachable rather
  than merely undocumented. A name arrives by being in your own file, by being
  exported from `core`, or by an `import` — of one name (`import time.Duration`)
  or of a whole module (`import time`). Using a private name says so and names
  the fix, rather than claiming the name does not exist.
- **Testing**: a test is a declaration named by a string —
  `test "an empty cart totals to zero" { ... }` — living in a companion file
  (`cart.test.sv` beside `cart.sv`) that is part of the module, so a test reaches
  its private declarations while nothing reaches the test. `salvo test` finds
  them, runs them and prints one report; assertions are ordinary functions that
  `throw`, so a vocabulary of your own is a function and nothing needs
  registering. A production build never loads a `.test.sv` file, so there is
  nothing to strip.
- **Documentation**: the `//` comment block above a declaration is its
  documentation — markdown, with `[symbol]` references to parameters,
  fields and types; struct fields, effect and handler members are
  documented individually. The language server shows these on hover,
  alongside a variable's type as narrowed at the cursor.

## Status

Work in progress, and deliberately unstable: the language is
experimental, so syntax and rules change without backwards-compatibility
guarantees — no deprecation periods, no dual-accepting grammars. Examples
in the repository and the specs are rewritten whenever a rule changes.
Both backends work end-to-end, verified by compiling and
running the output with `kotlinc` and `rustc`: the Rust backend derives
ownership mechanically — deductions decide whether parameters are moved or
borrowed (`&`/`&mut` via `Mut`), unions become enums, and effects become
traits. See [COMPLETED.md](COMPLETED.md) and [ROADMAP.md](ROADMAP.md) for details and
[AGENTS.md](AGENTS.md) if contributing.

## License

See [LICENSE](LICENSE).
