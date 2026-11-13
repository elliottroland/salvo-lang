# Generics and type aliases

## Overview

A **generic** declaration takes types as parameters, written in angle
brackets, so one declaration serves every element type: `List<T>` is a list of
anything, and `Pair<S, T>` below is a pair of any two types. A **type alias**
gives a long type a short name. Both will look familiar from Kotlin, Rust or
TypeScript:

```salvo
struct Pair<S, T> {
    first: S,
    second: T
}

fn swapped<S, T>(p: Pair<S, T>) -> Pair<T, S> => !p {
    return Pair { first: p.second, second: p.first }
}

let p = swapped(Pair { first: "one", second: 1 })   // a Pair<Int, Str>

type Outcome<S, T> = Ok S | Err T                   // a generic alias
```

The type arguments are usually inferred from the values, as in the call to
`swapped` above. Generic functions are covered with the rest of function
declarations in [Generic functions](Functions.md#generic-functions).

A generic parameter is opaque: with no bounds, nothing about a `T` is known.
Where generic code needs to do something with its values (compare them, add
them, print them), it asks for that capability as an implicit parameter, such
as `?cmp` or `?plus`, which the caller's scope supplies
([Implicit parameters](Implicit-Parameters.md)).

The rest of this page covers generic types, the qualifiers that make a
`Result`-like type out of generics, generic arms in a union, and aliases.

## Generic types

Structs, qualifiers and arrays can all refer to generic types, with the
following syntax:

```salvo
// Struct with generic types S and T
struct Pair<S, T> {
    first: S,
    second: T
}

// A qualifier can be as generic as the type, or less. This governs which `Pair`s the qualifier can apply to.
qualifier Something<S, T> for Pair<S, T>
qualifier FirstStr<T> for Pair<Str, T>
qualifier Ints for Pair<Int, Int>
```

The `<` touches the name it belongs to, as an argument list's `(` does: at a
declaration (`struct Pair<S, T>`, `fn swapped<S, T>(…)`) and at a use
(`Pair<Int, Str>`, `swapped<Int, Str>(p)`). A space or a line break before it
is an error, so `List <Int>` is written `List<Int>`. In an expression a `<`
is the type arguments of what comes before it only when a type list, its `>`
and a `(` follow, as in `swapped<Int, Str>(p)`; otherwise it is less-than,
spaced or not (`a <b`, `a < b`). A type's arguments are written in full: a
`Pair<Int>` is an error, since a type position has no values to infer the
missing one from.

### A result type from generic qualifiers

Using qualifiers and generics we can implement the equivalent of Rust's
`Result` type. `std.core` ships exactly this pair,
[`Ok`](../../lib/std/core/result.sv#qualifier-ok) and
[`Err`](../../lib/std/core/result.sv#qualifier-err), so you do not have to
declare it. But nothing about it is built in, and the declarations are just:

```salvo
provenance qualifier Ok<T> for T
provenance qualifier Err<T> for T

let result: Ok Int | Err Str = get_age()

if result is Ok {
    // `result` is `Ok Int` here
    println("Age is ${result}")
} else { // No need to check that it's Err, because all other options are excluded by the other branches
    // `result` is `Err Str` here
    println("Something went wrong calculating age: ${result}")
}
```

A value is tagged `Ok` by `ok(x)` and `Err` by `err(x)`, so a function that
answers `Ok Int | Err Str` hands back one arm or the other, and `is` tells them
apart (see [std.core.result](../../lib/std/core/result.sv)). Qualifiers compose
freely ([Qualifiers](Qualifiers.md)), so nothing in the type system forbids a
value carrying both tags; the arms stay apart because each function mints one.

Because the qualifier's type is generic, the type it qualifies can itself be a
qualified type:

```salvo
let result: Ok (Ok Str | Err Int) | Err Bool = some_function()
```

This nesting is generally discouraged (taking it apart needs the re-typing
described in [Control flow](Control-Flow.md#dropping-a-qualifier-x--q-and-x---q)).
There is nothing wrong with having several qualifiers over various types,
though:

```salvo
let result: Ok Str | Err Str | Err Bool = some_function()
```

This lets code accumulate failure modes and handle them at a higher level.
Because the qualifiers are generic, Salvo also supports more precise checks on
such unions:

```salvo
let result: Ok Str | Err Str | Err Bool = some_function()

if result is Err Str {
    // `result` is of type `Err Str`
}

if result is Err {
    // `result` is of type `Err Str | Err Bool`
}
```

## Generic union arms and spreads

A union type written in a signature may have **generic arms**. Each ordinary
arm, concrete or a bare generic, captures exactly one arm of the union it
meets:

```salvo
// Reorders the two arms of any two-arm union.
fn swap<T, S>(v: T | S) -> S | T => !v {
    when v {
        is T { return v }
        is S { return v }
    }
}
```

To let one generic arm capture **more than one** arm, write it as a spread
`...T`:

```salvo
struct Box<C> { value: C }

// `...T` absorbs every arm that is not a `Box<C>`.
fn detach<T, C>(value: ...T | Box<C>) -> T | C => !value {
    when value {
        is Box<C> { return value.value }
        is T { return value }
    }
}

// `T = Str | Int`, `C = Bool`; the result is the flat `Str | Int | Bool`.
let r = detach(some_str_int_or_box())
```

The arms the spread does not capture are matched first; whatever is left over
is bound to `...T`. The rules:

* A union may have **at most one** spread.
* The spread's type must be a bare generic in scope.
* A `...` is only ever a union arm.
* A bare generic arm that would have to absorb several arms (`T | Box<C>`
  meeting a three-arm union) does not match; reach for `...T` instead.

## Type aliases

Type aliases make union and tuple types more readable. An alias may be
generic:

```salvo
type Result<S, T> = Ok S | Err T
```

An alias is a name for the type it stands for, not a new type: it expands
where it is used, with its type arguments substituted, so `Result<Int, Str>`
and `Ok Int | Err Str` are the same type.

An alias of a **union** is also a declaration of its own: it may state
obligations and have a body of functions declared on it
([A named union's functions](Data-and-Types.md#a-named-unions-functions)).
An alias of any other type has no body.

Type aliases need to be imported like everything else
([Modules](Modules.md)).
