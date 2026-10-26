# Generics and type aliases

## Generic types

Structs, qualifiers, and arrays can all refer to generic types, using the following syntax:

```
// Struct with generic types S and  T
struct Pair<S, T> {
    first: S,
    second: T
}

// Qualifier can be as generic as type or less. This governs which `Pair`s the qualifier can apply to.
qualifier Something<S, T> of Pair<S, T>
qualifier FirstStr<T> of Pair<Str, T>
qualifier Ints of Pair<Int, Int>
```

Using qualifiers and generics we can implement the equivalent of a `Result` type from Rust. `std.core` ships exactly this pair, so you do not have to declare it — but nothing about it is built in, and the declarations are just:

```
provenance qualifier Ok<T> of T
provenance qualifier Err<T> of T

let result: Ok Int | Err Str = get_age()

if result is Ok {
    // `result` is `Ok Int` here
    println("Age is ${result}")
} else { // No need to check that it's Err, because all other options are excluded by the other branches
    // `result` is `Err Str` here
    println("Something went wrong calculating age: ${result}")
}
```

Because neither of these are `with` the other, it follows that nothing can be both at the same time. However, because the type of the qualifier is generic, it's possible for _it_ to be a qualified type:

```
let result: Ok (Ok Str | Err Int) | Err Bool = some_function()
```

This nesting is generally discouraged. There is nothing wrong with having multiple qualifiers over various types, such as:

```
let result: Ok Str | Err Str | Err Bool = some_function()
```

This allows us to accumulate failure modes and handle them at a higher level of the code. Because of generic qualifiers, Salvo also supports more precise checks for unions:

```
let result: Ok Str | Err Str | Err Bool = some_function()

if result is Err Str {
    // `result` is of type `Err Str`
}

if result is Err {
    // `result` is of type `Err Str | Err Bool`
}
```

### Generic union arms and spreads

A union type written in a signature may have **generic arms**. Each ordinary
arm — concrete or a bare generic — captures exactly one arm of the union it
meets:

```
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

```
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
is bound to `...T`. A union may have **at most one** spread, its type must be a
bare generic in scope, and a `...` is only ever a union arm. A bare generic
arm that would have to absorb several arms (`T | Box<C>` meeting a three-arm
union) no longer matches — reach for `...T` instead.

## Type aliases

Type aliases can be convenient for making union and tuple types more readable. You can use generics when declaring a type alias:

```
type Result<S, T> = Ok S | Err T
```

Type aliases need to be imported like everything else ([Modules](Modules.md)).
