# The Salvo language

Salvo is an experimental high-level language which compiles to both Rust and Kotlin and provides simple primitives for safe and easily-distributed software development. Salvo is an acronym which represents the interlocking values that the language seeks to provide:

* **Simplicity**: Things should have few parts (not complex) and easy to understand (not complicated), but if we must choose one then the latter: each part should be independently intelligible. When there are many parts, we shouldn't need to understand everything in order to understand anything. Salvo has a minimal syntax to avoid having a million ways of doing the same thing, and the more complicated features are opt-in so that you can gradually learn and integrate with them.
* **Affordancy**: It should be clear what can and will happen when an action is taken. The contribution a change makes to the system should be clear. Salvo has a type system which exposes what you can do with data via "qualifiers". These qualifiers allow users to encode unique behaviors and freely annotate types with references to functions and local variables.
* **Locality**: The more assumptions two parts of a system make about one another, the closer they should be. The fewer assumptions they make about one another, the more decoupled they should be. Salvo has no hidden state, and uses algebraic effects to capture what a function can do when executing.
* **Verifiability**: The correctness of a system or change should be provable. The design of the system and signals produced by it should be such that this proof of easy. Salvo has a strong static type system, including things like linear types (for ensuring that data is not accidentally dropped), exhaustive whens over arbitrary unions, a deduction system on functions, and more. Salvo makes testing a first-class citizen, built into the compiler around its qualifiers and actor-based concurrency.
* **Orthogonality**: The system should be divided into components which solve distinct problems and can change independently from one another. Salvo's features compose naturally and solve independent problems. Structs hold data, function define behavior, qualifiers refine data types, and effects carry capabilities for functions.

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

* **`LANGUAGE_SPEC.md`**, at the repository's root — the same features as
  short *labeled rules* (`[qual-erasure]`, `[deduce-syntax]`), plus the
  compiler decisions under each. Written for whoever changes the compiler; a
  reader learning Salvo does not need it.
* **The wiki** — these pages and a page per standard-library module, generated
  by `salvo docs` (run in `lib/`; `lib/salvo.toml`'s `[docs]` section says
  what goes in). Never edit it; edit these pages, or the doc comments in
  `lib/std`.

## Reading order

Start at the top; each page stands on its own, so jumping in is fine too.

### The language

| Page | What it covers |
|---|---|
| [Data and Types](Data-and-Types.md) | primitives, strings, tuples, unions, structs, and nullability without `null` |
| [Collections](Collections.md) | `List`, `Set`, `Map` and their sorted kin, arrays, `Any` and `Never` |
| [Qualifiers](Qualifiers.md) | type-level claims — `Mut`, predicates, constructive, state versus provenance, refinements |
| [Dependent Qualifiers](Dependent-Qualifiers.md) | claims about one value's relation to another (`Idx`, `KeyOf`, `NotSame`), and the total operations that consume them |
| [Generics and Aliases](Generics-and-Aliases.md) | generic types and type aliases |
| [Control Flow](Control-Flow.md) | `if`, `when`, `while`, `for`, and dropping a qualifier with `x: -Q` |

### Functions and behaviour

| Page | What it covers |
|---|---|
| [Functions](Functions.md) | declaration syntax, overload resolution |
| [Implicit Parameters](Implicit-Parameters.md) | capabilities the compiler supplies: `?` parameters, `fns` bundles, obligations on a struct, and orderings carried in a type (`Heap<T>(?cmp)`) |
| [Lambdas and Variadics](Lambdas-and-Variadics.md) | function values and variadic arguments |
| [Iteration](Iteration.md) | `Next`, and `fns` groups as a capability bundle |
| [Iterators](Iterators.md) | how `for` really works: an iterator struct, its `next`, `iter fn`, sources and combinators |
| [Comparison and Hashing](Comparison-and-Hashing.md) | equality, ordering and hashing as declared capabilities, and `by auto` |
| [Compile-Time Functions](Compile-Time-Functions.md) | `comptime fn` and `by`: the structural implementations written once, over the fields of any struct or the arms of any union |

### Guarantees

| Page | What it covers |
|---|---|
| [Deductions and Ownership](Deductions-and-Ownership.md) | the deduction clause, moves, projections, shared fate and `copy` |
| [Mutable Handles](Mutable-Handles.md) | handles into storage you do not own: mutable elements, lending, proven-distinct pairs (`NotSame`), and handles that share a container (`ref(c)`) |
| [Linear Types](Linear-Types.md) | values that must be used |
| [Throwing](Throwing.md) | leaving early with a message, and `try` |
| [Effects and Handlers](Effects-and-Handlers.md) | capabilities, `use`, dependencies, interception, monitors, mixed handlers |
| [Command-line tools](Command-Line.md) | a struct as a command line: `FromArgs<self> by args`, flags, positionals, help and errors |
| [Concurrency](Concurrency.md) | where work runs: pools, actors and waiting |

### The surrounding world

| Page | What it covers |
|---|---|
| [Modules](Modules.md) | files as modules, `export`, `import`, naming and documentation comments |
| [The Manifest](Manifest.md) | `salvo.toml`: projects, build settings, dependencies, host libraries, and every valid option |
| [Testing](Testing.md) | `test` declarations, where they live, and assertions |
| [Files](Files.md) | the filesystem surface and its streams |
| [Time](Time.md) | durations, instants, ticks, and time in a test |
| [Backends](Backends.md) | `intrinsic`, `platform`, host implementations, and per-backend detail |

### Inside the compiler

| Page | What it covers |
|---|---|
| [Dumps](Dumps.md) | `salvo dump`: what a phase made of your program, and how a dump spells names; the resolve phase's dump |
| [The IR](The-IR.md) | the decided form both backends read: how to read `salvo dump --phase generate-ir`, its declarations, statements and expressions, ownership marks, narrowing, loops, effects and actors |
