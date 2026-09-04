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
specification and [PROGRESS.md](PROGRESS.md) for implementation status.

## Building

```bash
cargo build        # build the compiler
cargo test         # run the test suite

# Type-check a directory of .sv sources without generating code:
cargo run -- analyze --src ./my_project              # or --format json

# Start a language server (LSP over stdio) for editor integration:
cargo run -- lsp
# A VS Code extension bundling syntax highlighting and the language server
# lives in vscode/ — see vscode/README.md.

# Compile a directory of .sv sources to Kotlin:
cargo run -- compile --backend kotlin --src ./my_project --target ./out

# Compile and run the output (requires kotlinc on PATH):
kotlinc out/*.kt out/core/*.kt -d classes && kotlin -cp classes salvo.main.MainKt

# Or compile the same sources to Rust:
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
  (destructuring and positional reads, `t.0`), arrays, immutable strings
  with `${}` interpolation.
- **Unions & nullability**: `A | B` types, `T?` as `T | None` (no null
  value), flow-sensitive narrowing via `is` — of variables and of field
  chains (`p.address.city`) — and exhaustive `when`.
- **Qualifiers**: type-level annotations (`Ok T`, `Surname Person`) enabling
  overloading, union tagging, and precise checks; `Mut` opts structs and
  types (`canbe Mut`) into mutability. A qualifier is a claim about a
  value's *contents* (`NonEmpty`) or about where the *handle* came from
  (`provenance qualifier Authenticated of Request`) — only the former can
  be invalidated by mutation. Structs and qualifiers can be namespaced
  under a struct (`Environment.Id`), giving wrapper types without nesting.
- **Everything is an expression**: `if`/`when` produce values; branch types
  union together.
- **Scope exits**: `defer { ... }` runs a block when the enclosing block
  ends — at its end and at every `return`/`break`/`continue` that leaves
  it, latest first — so a resource is released once, on every path.
- **Functions**: overloading by argument types, dot-notation
  (`list.size()` ≡ `size(list)`), variadics, lambdas, generics, and
  `yield`-based iterator functions.
- **Algebraic effects**: effects declare capabilities, handlers implement
  them, `use` registers handlers in scope — dependencies are always visible
  in signatures. A handler may itself depend on another effect (declared as
  a constructor parameter of effect type); the compiler supplies it from
  the enclosing scope, so callers never mention it.
- **Non-resumption**: a function that may leave early declares
  `[Abort<Str>]` and keeps its own return type; `abort(message)` returns
  `Nothing`, so intermediate frames stay silent. The delimiter is
  `try { ... }`, whose value is `Ok T | Aborted M` — an ordinary union, so
  `when` reads it like any result.
- **Deductions**: `-> [list: Mut] T` annotations describing what a function
  does to its parameters — the ownership contract for the Rust backend.
- **Interop**: `external`/`define` blocks map std functions onto native code
  per backend.
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
traits. See [PROGRESS.md](PROGRESS.md) for details and
[AGENTS.md](AGENTS.md) if contributing.

## License

See [LICENSE](LICENSE).
