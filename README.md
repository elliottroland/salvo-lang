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
  the whole immutable surface by dropping its `Mut`.
- **Collections**: `List`, `Set`, `Map` and their sorted counterparts, each
  with a literal — `[1, 2, 3]`, `{"a", "b"}`, `{"k": "v"}`, and `Mut` in
  front for a mutable one. `Set` and `Map` iterate in **insertion order on
  every backend**, so a program's output does not depend on the target it
  was compiled for. Every struct compares with `==`; a struct becomes a key
  by declaring `canbe hashed` (or `canbe ordered`, which also gives it `<`),
  checked where it is declared. Arrays stay for fixed-size data and the
  variadic boundary.
- **Unions & nullability**: `A | B` types, `T?` as `T | None` (no null
  value), flow-sensitive narrowing via `is` — of variables and of field
  chains (`p.address.city`) — and exhaustive `when`.
- **Qualifiers**: type-level annotations (`Ok T`, `Surname Person`) enabling
  overloading, union tagging, and precise checks; `Mut` opts structs and
  types (`canbe Mut`) into mutability. `is` narrows a value to a more
  specific type, `^` widens it by removing a qualifier (`when o { ^ Ok { … } }`
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
  in signatures. A handler may itself depend on another effect (declared as
  a constructor parameter of effect type); the compiler supplies it from
  the enclosing scope, so callers never mention it. A function *value* that
  performs an effect declares it in its type (`(s: Str) [Logger] -> Str`),
  and the effect is supplied by whoever calls the value — so a higher-order
  function inherits its callback's effects and needs no annotation of its
  own.
- **Non-resumption**: a function that may leave early declares
  `[Throw<Str>]` and keeps its own return type; `throw(message)` returns
  `Nothing`, so intermediate frames stay silent. The delimiter is
  `try { ... }`, whose value is `Ok T | Thrown M` — an ordinary union, so
  `when` reads it like any result.
- **Deductions**: a clause after the return type — `-> T => list: Mut` —
  describing what a function does to its parameters and what its result
  borrows of them; whatever it leaves unsaid is inferred from the body. The
  ownership contract for the Rust backend, and the one place Salvo states a
  borrow: `proj[from: list] T` returns an element without copying it, a
  struct with `proj` fields is a view, and a copy happens only where the
  program writes `copy`.
- **Interop**: a `platform effect` declares what the program needs from its
  target language; the compiler generates the interface and
  `salvo platform generate` writes the host implementation skeleton into
  `platform/`, so the *target's* compiler checks the two against each other.
  It is the only interop path: std's own primitives are `intrinsic`, lowered
  by code inside each backend, and `intrinsic` is the compiler's to declare.
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
