# The Salvo language

Salvo lang is an experimental high level programming language with C-like syntax which can transpile into Rust and Kotlin. The aim of the language is to support the transition from the JVM to a non-garbage collected language by using a language we can work with both of them. This has the following goals:

* No explicit pointer or references.
* No explicit ownership or borrowing, except possibly in type signatures.
* An imperative style rather than a purely-functional style.
* A strong type system, with algebraic effects.
* Support for "qualities" which allow us to annotate data in the type system.

The compiler is written in Rust, and translates code into an intermediate representation. From there, each backend language (initially Kotlin and Rust) writes this out to the respective language.

This directory is the **narrative specification**: what the language is and
how to use it. It is the source of truth for behaviour — when something is
ambiguous, these pages decide.

Two companions, with different jobs:

* **`../../LANGUAGE_SPEC.md`** — the same features as short *labeled rules*
  (`[qual-erasure]`, `[deduce-syntax]`), plus the compiler decisions under
  each. Written for whoever changes the compiler; a reader learning Salvo
  does not need it.
* **`../../wiki/`** — a generated copy of these pages for reading on GitHub
  (`tools/sync-wiki.sh`). Never edit it; edit here.

## Reading order

Start at the top; each page stands on its own, so jumping in is fine too.

### The language

| Page | What it covers |
|---|---|
| [Data and Types](Data-and-Types.md) | primitives, strings, tuples, unions, structs, and nullability without `null` |
| [Collections](Collections.md) | `List`, `Set`, `Map` and their sorted kin, arrays, `Any` and `Never` |
| [Qualifiers](Qualifiers.md) | type-level claims — `Mut`, predicates, constructive, state versus provenance, dependent claims, refinements |
| [Generics and Aliases](Generics-and-Aliases.md) | generic types and type aliases |
| [Control Flow](Control-Flow.md) | `if`, `when`, `while`, `for`, and lifting a qualifier with `is ^Q` |

### Functions and behaviour

| Page | What it covers |
|---|---|
| [Functions](Functions.md) | declaration syntax, overload resolution, implicit parameters |
| [Lambdas and Variadics](Lambdas-and-Variadics.md) | function values and variadic arguments |
| [Iteration](Iteration.md) | `Yield`, and `params` groups as a capability bundle |
| [Passes](Passes.md) | how `for` really works: a pass, its `next`, and `iter fn` |
| [Comparison and Hashing](Comparison-and-Hashing.md) | equality, ordering and hashing as declared capabilities |

### Guarantees

| Page | What it covers |
|---|---|
| [Deductions and Ownership](Deductions-and-Ownership.md) | the deduction clause, moves, projections, shared fate and `copy` |
| [Mutable Handles](Mutable-Handles.md) | handles into storage you do not own: mutable elements, lending, proven-disjoint pairs (`NotEq`), and declared aliasing (`canbe`) |
| [Linear Types](Linear-Types.md) | values that must be used |
| [Throwing](Throwing.md) | leaving early with a message, and `try` |
| [Effects and Handlers](Effects-and-Handlers.md) | capabilities, `use`, dependencies, interception, monitors, mixed handlers |
| [Concurrency](Concurrency.md) | where work runs: pools, actors and waiting |

### The surrounding world

| Page | What it covers |
|---|---|
| [Modules](Modules.md) | files as modules, `export`, `import`, naming and documentation comments |
| [Testing](Testing.md) | `test` declarations, where they live, and assertions |
| [Files](Files.md) | the filesystem surface and its streams |
| [Time](Time.md) | durations, instants, ticks, and time in a test |
| [Backends](Backends.md) | `intrinsic`, `platform`, host implementations, and per-backend detail |
