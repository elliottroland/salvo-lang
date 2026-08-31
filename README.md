# Salvo

Salvo is an experimental high-level programming language with C-like syntax
that transpiles to **Kotlin** and (eventually) **Rust**. It supports the
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

# Compile a directory of .sv sources to Kotlin:
cargo run -- compile --backend kotlin --src ./my_project --target ./out

# Compile and run the output (requires kotlinc on PATH):
kotlinc out/*.kt out/core/*.kt -d classes && kotlin -cp classes salvo.MainKt
```

## A taste of Salvo

```
struct Person {
    name: Str,
    surname: Str? = None,   // Str? is shorthand for Str | None
    age: Int
}

qualifier Ok<T> of T
qualifier Err<T> of T

// `-> T as Ok` marks a constructor function: the returned value gains
// the qualifier by construction, and callers see `Ok T`.
fn ok<T>(value: T) -> T as Ok {
    return value
}

fn err<T>(value: T) -> T as Err {
    return value
}

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

- **Data**: structs (defaults, spread `...`, destructuring), tuples, arrays,
  immutable strings with `${}` interpolation.
- **Unions & nullability**: `A | B` types, `T?` as `T | None` (no null
  value), flow-sensitive narrowing via `is`, exhaustive `when`.
- **Qualifiers**: type-level annotations (`Ok T`, `Surname Person`) enabling
  overloading, union tagging, and precise checks; `Mut` opts structs into
  mutability.
- **Everything is an expression**: `if`/`when` produce values; branch types
  union together.
- **Functions**: overloading by argument types, dot-notation
  (`list.size()` ≡ `size(list)`), variadics, lambdas, generics, and
  `yield`-based iterator functions.
- **Algebraic effects**: effects declare capabilities, handlers implement
  them, `use` registers handlers in scope — dependencies are always visible
  in signatures.
- **Deductions**: `-> [list: Mut] T` annotations describing what a function
  does to its parameters — the ownership contract for the Rust backend.
- **Interop**: `external`/`define` blocks map std functions onto native code
  per backend.

## Status

Work in progress. The Kotlin backend works end-to-end (verified by compiling
and running the output with `kotlinc`); the Rust backend is planned. See
[PROGRESS.md](PROGRESS.md) for details and [AGENTS.md](AGENTS.md) if
contributing.

## License

See [LICENSE](LICENSE).
