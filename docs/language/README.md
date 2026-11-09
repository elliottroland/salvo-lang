# The Salvo language

Salvo is an experimental high-level language that compiles to both Rust and
Kotlin. You write ordinary imperative code, with structs, functions and
loops, and no pointers, references or lifetimes. The compiler works out what
is borrowed and what is moved, and writes each target's own idiom: borrows
and moves for Rust, plain references for Kotlin. One program runs on the JVM
or natively, with the same output.

What Salvo adds to that familiar surface is a type system that records more
of what you know about a program:

- **Unions and exhaustive `when`**: a value is one of several types, and
  every case must be handled. There is no `null`; an absent value is `T?`.
- **Qualifiers**: claims carried in a type. A `Positive Int` has been
  tested, a `NonEmpty List` has an element, an `Idx(xs) Int` is an index
  that is in range for `xs`. Functions demand claims, and the compiler
  tracks where they hold.
- **Effects**: what a function may do (print, read files, read the clock,
  throw) is part of its signature, and a handler supplies it. A test swaps
  in an in-memory file system or a virtual clock without touching the code.
- **Linear types**: a value that must be used, such as an open file or a
  promised reply, cannot be forgotten. The program does not compile until
  every path closes or hands it on.
- **Actors**: concurrency is state that runs one message at a time on a pool
  of threads, on one machine or several.

## A first look

```salvo
// A shape is one of two structs; `when` must cover both.
struct Circle { radius: Double }
struct Rect { w: Double, h: Double }
type Shape = Circle | Rect

fn area(s: Shape) -> Double {
    when s {
        is Circle { return 3.14159 * s.radius * s.radius }
        is Rect { return s.w * s.h }
    }
}

// A qualifier is a claim the compiler tracks: a `Positive Int` has been
// tested, so `share` cannot be handed zero.
qualifier Positive for Int {
    fn qualifies(n: Int) -> Bool {
        return n > 0
    }
}

fn share(total: Double, people: Positive Int) -> Double {
    return total / to_double(people)
}

// `main` says it uses effects; `use` supplies the console.
fn main() [use] {
    use StdConsole()
    let c: Shape = Circle { radius: 1.0 }
    let r: Shape = Rect { w: 2.0, h: 3.0 }
    let shapes = List(c, r)
    let total = reduce(shapes, 0.0, (acc, s) -> acc + area(s))
    let n = size(shapes)
    if n is Positive {
        println("each: ${share(total, n)}")             // each: 4.570795
    }
}
```

`salvo run --backend rust` and `salvo run --backend kotlin` print the same
line. Calling `share(total, size(shapes))` without the test is a compile
error: nothing has shown that the size is positive. The pages below explain
each piece: [Data and Types](Data-and-Types.md) for structs and unions,
[Qualifiers](Qualifiers.md) for `Positive`, [Effects and
Handlers](Effects-and-Handlers.md) for `use`, and
[Collections](Collections.md) and [Iteration](Iteration.md) for `List` and
`reduce`.

## The values behind it

Salvo is an acronym for the values the language tries to serve:

* **Simplicity**: Things should have few parts (not complex) and be easy to
  understand (not complicated), but if we must choose one then the latter:
  each part should be independently intelligible. When there are many
  parts, we shouldn't need to understand everything in order to understand
  anything. Salvo has a minimal syntax to avoid having a million ways of
  doing the same thing, and the more complicated features are opt-in, so
  you can learn and adopt them gradually.
* **Affordancy**: It should be clear what can and will happen when an action
  is taken, and what a change contributes to the system. Salvo's type system
  exposes what you can do with data through qualifiers, which let you encode
  behaviours and annotate types with references to functions and local
  variables.
* **Locality**: The more assumptions two parts of a system make about one
  another, the closer they should be; the fewer they make, the more
  decoupled they can be. Salvo has no hidden state, and uses algebraic
  effects to capture what a function can do when it runs.
* **Verifiability**: The correctness of a system or a change should be
  provable, and the system's design and signals should make that proof easy.
  Salvo has a strong static type system, with linear types (so data is not
  accidentally dropped), exhaustive `when` over arbitrary unions, a
  deduction system on functions, and more. Testing is built in, around its
  qualifiers and its actor-based concurrency.
* **Orthogonality**: The system should be divided into components that solve
  distinct problems and can change independently. Salvo's features compose:
  structs hold data, functions define behaviour, qualifiers refine types,
  and effects carry capabilities.

Salvo began as a way to move code from the JVM to a language without a
garbage collector while working in one language for both. Its goals follow
from that: no explicit pointers or references; no explicit ownership or
borrowing, except possibly in signatures; an imperative style rather than a
purely functional one; a strong type system with algebraic effects; and
qualifiers for annotating data in the type system. The compiler, written in
Rust, translates a program into an intermediate representation, and each
backend writes that out in its own language.

## About these pages

These pages are the **narrative specification**: what the language is and
how to use it, and the source of truth for behaviour. When something is
ambiguous, these pages decide. Each page opens with an overview of its
feature; the sections after it cover the details, caveats and extensions,
in order of need.

Two companions have different jobs:

* **The standard library's own documentation**, a page per module, written
  from the doc comments in `lib/std`: what each type and function does, with
  examples. The pages below link into it, for example
  [`std.core.list`](../../lib/std/core/list.sv) or
  [`std.core.actor`](../../lib/std/core/actor.sv).
* **`LANGUAGE_SPEC.md`**, at the repository's root: the same features as
  short *labeled rules* (`[qual-erasure]`, `[deduce-syntax]`), plus the
  compiler decisions under each. It is written for whoever changes the
  compiler; a reader learning Salvo does not need it.

The wiki is generated from both, by `salvo docs` run in `lib/` (its
`salvo.toml`'s `[docs]` section says what goes in). Never edit the wiki;
edit these pages or the doc comments in `lib/std`.

## Reading order

Start at the top; each page stands on its own, so jumping in is fine too.

### The language

| Page | What it covers |
|---|---|
| [Data and Types](Data-and-Types.md) | primitives, strings, tuples, unions, structs, and optional values without `null` |
| [Collections](Collections.md) | `List`, `Set`, `Map` and their sorted kin, deques, buffers, arrays, `Any` and `Never` |
| [Qualifiers](Qualifiers.md) | claims in a type: `Mut`, predicates, constructive qualifiers, state versus provenance, refinements |
| [Dependent Qualifiers](Dependent-Qualifiers.md) | claims about one value's relation to another (`Idx`, `KeyOf`, `NotSame`), and the total operations that consume them |
| [Generics and Aliases](Generics-and-Aliases.md) | generic types and type aliases |
| [Control Flow](Control-Flow.md) | `if`, `when`, `while`, `for`, and dropping a qualifier with `x: -Q` |

### Functions and behaviour

| Page | What it covers |
|---|---|
| [Functions](Functions.md) | declaration syntax, dot calls, defaults, overload resolution |
| [Implicit Parameters](Implicit-Parameters.md) | capabilities the compiler supplies: `?` parameters, `fns` bundles, obligations on a struct, and orderings carried in a type (`Heap<T>(?cmp)`) |
| [Lambdas and Variadics](Lambdas-and-Variadics.md) | function values, effect variables and variadic arguments |
| [Iteration](Iteration.md) | using `for` and the sequence functions: eager `map`/`filter`/`reduce`, lazy adaptors, combinators over `Next` |
| [Iterators](Iterators.md) | how `for` really works: the protocol, writing an iterator struct or an `iter fn`, sources |
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
| [Command-line tools](Command-Line.md) | a struct as a command line (`FromArgs<self> by args`), and the raw arguments, exit status and environment |
| [Concurrency](Concurrency.md) | actors, tasks, pools and waiting, on one machine and across several |

### The surrounding world

| Page | What it covers |
|---|---|
| [Modules](Modules.md) | files as modules, `export`, `import`, namespaces, and documentation comments |
| [The Manifest](Manifest.md) | `salvo.toml`: projects, build settings, dependencies, host libraries, and every valid option |
| [Testing](Testing.md) | `test` declarations, where they live, assertions, and actor tests |
| [Writing Test Kinds](Test-Kinds.md) | `comptime fn` test kinds: the `Test` parameter, arguments, verdicts, and writing your own |
| [Files](Files.md) | the file system and its streams |
| [Time](Time.md) | durations, instants, ticks, and time in a test |
| [Backends](Backends.md) | `intrinsic`, `platform`, host implementations, and per-backend detail |

### Inside the compiler

| Page | What it covers |
|---|---|
| [Dumps](Dumps.md) | `salvo dump`: what each phase made of your program, and how a dump spells names |
| [The IR](The-IR.md) | the decided form both backends read: how to read `salvo dump --phase generate-ir`, its declarations, statements and expressions, ownership marks, narrowing, loops, effects and actors |
