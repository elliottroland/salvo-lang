# Data and Types

## Overview

Salvo describes data with a few building blocks that combine freely:

* **scalars**: numbers, `Bool`, `Char` and `Byte`;
* **strings** (`Str`) and byte buffers (`Bytes`);
* **tuples**, which hold several values in order: `(Str, Int)`;
* **unions**, which hold one value of several possible types: `Str | Int`;
* **structs**, records of named fields.

There are no classes and no methods. A struct is plain data, like a Kotlin
data class or a Rust struct with public fields, and behaviour lives in
functions that take it (`size(xs)` can also be written `xs.size()`; see
[Dot calls](Functions.md#dot-calls)). There is no `null` either: a value that may be
absent has a union type with `None` in it, written `T?` for short, and the
compiler makes you check it before use, much as Kotlin's nullable types and
TypeScript's `T | undefined` do. Values are immutable unless their type says
`Mut` ([Qualifiers](Qualifiers.md)).

```salvo
struct Person {
    name: Str,
    nickname: Str? = None,      // optional: absent unless given
    age: Int,
}

fn greeting(p: Person) -> Str {
    let nick = p.nickname ?: p.name           // `?:` supplies a default for `None`
    return "hello ${nick}, aged ${p.age}"
}

fn describe(value: Int | Str) -> Str {
    if value is Int {
        return "the number ${value + 1}"      // `value` is an `Int` here
    }
    return "the text \"${value}\""            // and a `Str` here
}
```

`is` tests which arm of a union a value holds, and inside the test the value
reads as that arm; that is called *narrowing*, and it is how every union is
taken apart. The rest of this page goes through each kind of data in turn:
the scalars and their arithmetic, strings, tuples, unions, structs, and the
optional values built from unions.

## Basic data types

Salvo has the following basic data types, similar to the JVM's:

| Type | What it is | Kotlin | Rust |
|---|---|---|---|
| `Byte` | an 8-bit unsigned integer, 0..255 | `UByte` | `u8` |
| `Int` | a 32-bit signed integer | `Int` | `i32` |
| `Long` | a 64-bit signed integer | `Long` | `i64` |
| `Float` | a 32-bit floating-point number | `Float` | `f32` |
| `Double` | a 64-bit floating-point number | `Double` | `f64` |
| `Bool` | a boolean | `Boolean` | `bool` |
| `Char` | a character | `Char` | `char` |
| `None` | a singleton type, for an expression with no response | | |

`Byte`, `Int`, `Long`, `Float`, `Double`, `Bool` and `Char` are the **basic
types**: scalars whose operators are the host language's own. An operator is
the call it names (`eq(a, b)`, `cmp(a, b)`, `plus(a, b)`), and std's functions
for the scalars are written as the host's operators (see
[Comparison, equality and hashing](Comparison-and-Hashing.md#what-an-operator-calls)).
`Bytes`, `None` and `Str` (below) are not basic types, so every operation on
them is a function call. The scalars and their conversions are documented in
[std.core.basic](../../lib/std/core/basic.sv).

### Bytes

A `Byte` is an octet rather than a number: the arithmetic operators do not take
it. [`to_int(b)`](../../lib/std/core/basic.sv#fn-to_int) and
[`to_byte(n)`](../../lib/std/core/basic.sv#fn-to_byte) convert, and `to_byte`
keeps the low 8 bits.

[`Bytes`](../../lib/std/core/bytes.sv#type-bytes) is a **byte buffer**, and the
currency of every byte-shaped API in std. It is deliberately not a
`List<Byte>`, which would box every octet on the JVM and carry a list's surface
rather than a buffer's. Like `Str` it comes in two shapes:

* `Bytes` is read: `size`, `get`, `slice`, `index_of`, `to_str`, `to_hex`,
  `str_of_bytes`, and `for b in data`;
* `Mut Bytes` is built: `add`, `append`, `set`, `clear`. It reaches the whole
  read surface by dropping its `Mut`.

`bytes_of(...)` and `mut_bytes(...)` construct one. `to_bytes(str)` and
`str_of_bytes(data)` bridge to text, in UTF-8, and the decoding is strict. The
module page, [std.core.bytes](../../lib/std/core/bytes.sv), has examples.

## Arithmetic

All the numbers have the usual arithmetic: `+`, `-`, `*`, `/`, `%` and `**`
work on `Int`, `Long`, `Float` and `Double`, and so does unary `-` (`**` is
described below). `/` between integers is integer division on every backend.

```salvo
let count = 42          // Int, the default for a whole number
let total = 42L         // Long
let ratio = 1.5         // Double, the default for a fraction
let small = 1.5f        // Float
let ok = true           // Bool
let initial = 'S'       // Char

let widened = count + total          // computes at Long: 84
let half = 7 / 2                     // integer division: 3
```

Two things other languages allow are not allowed here. `+` does not
concatenate strings: `${}` interpolation is how text is built. And `&&`, `||`
and `!` take `Bool` operands only: Salvo has no truthiness in value position,
any more than in conditions.

### Operators are function calls

Like ordering and equality, the arithmetic operators are **the calls they
name**:

| Written | Means |
|---|---|
| `a + b` | `plus(a, b)` |
| `a - b` | `minus(a, b)` |
| `a * b` | `times(a, b)` |
| `a / b` | `divide(a, b)` |
| `a % b` | `rem(a, b)` |
| `-a` | `neg(a)` |
| `a ** b` | `pow(a, b)` |

Each is chosen exactly as the written call would be. The numbers' functions are
std's, declared beside the types, and the backends write the host's own
operator for them. A type of your own gets an operator by declaring the
function **in its own file**, with any right operand and any result:

```salvo
struct V { x: Int, y: Int }

fn plus(a: V, b: V) -> V { return V { x: a.x + b.x, y: a.y + b.y } }
fn times(a: V, k: Int) -> V { return V { x: a.x * k, y: a.y * k } }

let w = (a + b) * 3
```

An implicit parameter of the function you are in comes first, as it does for
every call: inside `fn total<T>(a: T, b: T, ?plus: (a: T, b: T) -> T)`, `a + b`
is that `?plus`. That is the only way generic code can add, and the error at a
`T` without one says to ask for it. A member `plus` of an available effect is a
candidate just as a function is. Ordering (`<`, `<=`, `>`, `>=`) and equality
(`==`, `!=`) work the same way: `a < b` means `cmp(a, b) < 0` and `a == b`
means `eq(a, b)` (see [Comparison, equality and hashing](Comparison-and-Hashing.md)).

`++` and `--` name no function: they step an `Int` place by one.

### Power

`**` is the power operator. It binds tighter than `*` and than a `-` on its
left, and it is right-associative: `-2 ** 2` is `-4`, `2 ** 3 ** 2` is
`2 ** 9`, and the exponent may be negative (`2.0 ** -1`). Nothing is widened,
since an exponent is not a base: std has `Int ** Int`, `Long ** Int`,
`Double ** Int` and `Double ** Double`. An integer power wraps like the rest of
integer arithmetic, and a negative integer exponent is an error at run time,
as a division by zero is. `Double ** Int` is computed by repeated
multiplication, identically on every backend, and `Double ** Double` is the
host's `pow`.

### Compound assignment

`x += e` and its siblings (`-=`, `*=`, `/=`, `**=`) are shorthand for
`x = x + e`, so they follow the same operand rules: `+=` on a `Str` is refused
as `+` is, and `v += w` on a type with a `plus` calls it. They are statements
rather than expressions, so they do not chain. `x++` and `x--` remain for a
step of one, where the value of the expression matters.

An assignment's left side must be a *place*: a variable, a field path, or a
subscript.

### Widening and conversions

Mixed widths widen implicitly **within** a class: `Int + Long` computes at
`Long`, and `Float < Double` compares at `Double`. The compiler records the
promotion and each backend renders it its own way. An operator widens before it
calls its function, so `i + l` is `plus` at `Long` and `i < l` compares at
`Long`.

Mixing the integer and the floating-point classes never happens implicitly:
`1 + 2.5` is a type error naming the explicit conversions.
[`to_int`](../../lib/std/core/basic.sv#fn-to_int),
[`to_long`](../../lib/std/core/basic.sv#fn-to_long),
[`to_float`](../../lib/std/core/basic.sv#fn-to_float) and
[`to_double`](../../lib/std/core/basic.sv#fn-to_double) are declared in
`std.core.basic` for every source width. They truncate toward zero and saturate
at the target's bounds, identically on both backends, except that
`to_int(Long)` keeps the low 32 bits.

### Numeric literals

Numeric literals default to `Int` and `Double`: `1` is an `Int` and `1.2` is a
`Double`. Suffixes select the other widths: `1L` is a `Long`, and `1.2f` is a
`Float`. The `f` suffix requires a decimal point, so write `1.0f`, not `1f`.
Underscores may separate digits (`1_000_000L`).

An **unsuffixed** literal also *adopts* the numeric type its position expects:
`let x: Long = 1`, `let d: Double = 3` and passing `1` to a `Long` parameter all
work, and `x + 1` needs no `1L` because operator widening covers it. Adoption is
for literals only: an `Int` *variable* never becomes a `Long` implicitly, so
write `to_long(n)`.

### Overflow and bitwise operations

Integer arithmetic **wraps** on overflow: `Int` and `Long` `+`, `-`, `*`, `**`,
unary `-` and `++`/`--` keep the low 32 or 64 bits, the same on both backends,
so `2147483647 + 1` is `-2147483648`.

Bitwise operations are functions rather than operators, since `&`, `|` and `>>`
already mean other things in the grammar:
[`bit_and`](../../lib/std/core/basic.sv#fn-bit_and), `bit_or`, `bit_xor`,
`bit_not`, `bit_shl`, `bit_shr` (keeps the sign) and `bit_ushr` (fills with
zeros), each for `Int` and `Long`. A shift count is taken modulo the width, so
`bit_shl(1, 33)` is `2`.

### The text of a float

A float's **text** is the same on every backend: the shortest digits that
round-trip, always with a decimal point, and scientific notation outside
`[10^-3, 10^7)`. So `${2.0}` is `2.0`, `${0.0001}` is `1.0E-4`, `${1234567.0}`
is `1234567.0` and `${12345678.0}` is `1.2345678E7`, with `NaN`, `Infinity`
and `-Infinity` for the special values. That holds wherever a float becomes
text: on its own, inside a list or a map value, and as a struct field.

## Strings

Salvo strings work much as Kotlin's do: a `Str` is immutable, and a literal
makes one:

```salvo
let str: Str = "hello, world!"
```

There is no equivalent of Rust's string-slice type `str`. Text is put together
by **interpolation**: `"${expr}"` inside a literal inserts the text of a value.
The string functions (`size`, `split`, `trim`, `index_of`, `replace` and the
rest) are documented in [std.core.string](../../lib/std/core/string.sv), the
module that governs how strings behave.

### Building a string: `Mut Str`

A string being immutable does not mean building one has to be quadratic. `Str`
opts into the `Mut` auto-qualifier ([Qualifiers](Qualifiers.md)), so a
`Mut Str` is *a string under construction*. It is asked for explicitly (a
literal is never a `Mut Str`), and it is the only place the string functions
come in a mutating flavour:

```salvo
let text: Mut Str = Mut Str("hello")
append(text, ", world")
append(text, '!')          // one character
set(text, 0, 'H')          // answers whether it wrote; ignorable
println(text)              // Hello, world!
```

Everything else in `std.core.string` takes a plain `Str`, and a `Mut Str`
reaches all of it by *dropping* its `Mut`, as any qualifier can be dropped:
`size(text)`, `trim(text)` and `text == other` all work.

One difference from every other qualifier is invisible in Salvo and matters to
the backends: `Mut` is the one qualifier a target may render as a different
type (a `StringBuilder` on the JVM), so dropping it there is a real conversion
rather than a widening. The compiler records the drop and each backend renders
what it needs, which is what keeps `text == other` a comparison of
*characters* on both targets.

### Interpolation

Each `${value}` is the call `to_str(value)`, and it resolves exactly as that
call would if you wrote it there:

* the standard library has a `to_str` for every scalar and for `Str`;
* a type may declare its own;
* an effect in scope may provide one;
* a function's own `?to_str` parameter takes it first.

```salvo
struct Point { x: Int, y: Int }

fn to_str(p: Point) -> Str => p {
    return "(${p.x}, ${p.y})"
}

println("at ${point}")     // uses the `to_str` above
```

Without a text form, the compiler says so rather than letting the backend fail.
The standard library renders a list this way (`"${List(1, 2, 3)}"` gives
`[1, 2, 3]`), and a `Mut Str` needs nothing special, since it drops its `Mut`
first.

[`println`](../../lib/std/core/console.sv#fn-println) takes such a value
directly: `println(point)` prints what `to_str(point)` answers, found the same
way, so it says what `println("${point}")` does. A type with no text form is
an error at the call.

A union is the text of the arm it holds. It interpolates when every arm has a
text form of its own (a scalar, `Str`, or a type with a `to_str`), and a
`to_str` that takes the union itself takes precedence. `println(u)` needs that
`to_str`, so it prints a named union that declares one; an anonymous union
(`Int | Str`, or a `T?`) cannot have one, so write `println("${u}")`.

**An interpolation may hold a string literal**, and that literal may
interpolate in turn: `"${name ?: "unknown"}"` is an ordinary thing to write,
and the inner quotes belong to the inner literal. Escapes work inside it as
anywhere else.

### A type's text form

**A type's `to_str` lives in the type's own file.** `to_str` is what
interpolation calls, so a `to_str(p: Point)` may be declared only in the file
that declares `Point`. Anywhere else it is an error where it is written, used
or not, and the same goes for a catch-all `to_str<T>(x: T)`, which would be
every type's at once. Another rendering is a function with another name, passed
where it is wanted (`to_str = as_hex`). The rule is the same for every
operator; [Comparison, equality and hashing](Comparison-and-Hashing.md#one-per-type-where-an-operator-is-declared)
has it whole.

**A struct can get a generated `to_str`.** `: ToStr<self> by auto` writes a
`to_str` that renders the struct in the shape its literal has, identical on
both backends:

```salvo
struct Person : ToStr<self> by auto { name: Str, age: Int }
println("${Person {name: "ann", age: 3}}")    // Person { name: ann, age: 3 }
```

Each field is interpolated in turn, so a nested struct renders with its own
`to_str`, and needs one, or the error lands on that field. A `to_str` written
by hand is the other way in. The format is the language's own on purpose:
leaving it to each target would print different text on the JVM than in Rust.
How `by auto` works is [Compile-time functions](Compile-Time-Functions.md).

**Declaring the obligation is optional.** A type may say
[`: ToStr<self>`](../../lib/std/core/basic.sv#fns-tostr) without `by auto`.
That does not change how interpolation works: it checks at the declaration that
a matching `to_str` exists, so a missing one surfaces where the type is
defined rather than where it is printed.

### Reading a value back out of text

[`from_str`](../../lib/std/core/string.sv#fn-from_str) is the dual of `to_str`.
It returns `None` for text that does not spell a value, and it is overloaded by
its return type ([Functions](Functions.md#overloading-by-return-type)), so the
type the result goes to chooses the reader:

```salvo
let port: Int? = from_str("8080")       // 8080
let ratio: Double? = from_str("2.5e-1") // 0.25
let on: Bool? = from_str("yes")         // None: only `true` and `false`
```

`std.core.string` has it for `Str`, `Int`, `Long`, `Double` and `Bool`, and
under the names `parse_int`, `parse_long`, `parse_double` and `parse_bool`.
They accept the same text on every backend: an optional sign, digits, and for a
`Double` a fraction and an exponent. Whitespace, `NaN`, `inf`, hex and type
suffixes are refused everywhere, rather than accepted by whichever host happens
to. A generic function that needs a reader asks for `?FromStr<T>`.

## Tuples

Salvo has algebraic data types in the form of tuples (for AND) and unions (for
OR). The tuple of the types `A`, `B` and `C` is written `(A, B, C)`; the union
of them, described next, is `A | B | C`. A tuple can be destructured:

```salvo
// The type of `a` is Str
// The type of `b` is Int
// The type of `c` is Bool
let (a, b, c) = ("String", -1, true)
```

A pattern has to match what it destructures: the value must be a tuple, and a
tuple of exactly that many elements. `let (a, b) = 7` and
`let (a, b) = (1, 2, 3)` are both errors that say which. A loop binding is a
pattern too, checked against the element type:

```salvo
for (name, score) in iter(rows) {
    println("${name}: ${score}")
}
```

A single element can also be read by its position, written like a field with
the index in place of the name:

```salvo
let point: (Int, Int) = (3, 4)
let x = point.0             // Int
let y = point.1             // Int

// Nesting reads left to right, so `.0.1` is "element 1 of element 0".
let nested: ((Int, Int), Str) = ((3, 4), "label")
let inner_y = nested.0.1    // Int
```

Positions are counted from zero, and an index that the tuple does not have is
a compile-time error: unlike an array subscript, the position is part of the
program text rather than a value computed at run time.

Tuple elements are read-only. Qualifiers never apply to a tuple, so no tuple
value can be `Mut`, and there is nothing to assign through. Build a new tuple
instead.

## Unions

A union `A | B | C` holds a value of exactly one of its *arms*. Unions are
tested with `is`, in `if` conditions and `while` loops, with syntax similar to
type tests in C#:

```salvo
let string_or_number: Str | Int = func()
if string_or_number is Str s {
    // The type of `string_or_number` is `Str` in this block
}

while string_or_number is Int i {
    // The type of `string_or_number` is `Int`
    string_or_number = func()
}
```

`is Str s` both tests and binds: `s` names the value as a `Str`. The `when`
expression ([Control flow](Control-Flow.md#when-expressions)) takes a union
apart arm by arm and checks that every arm is handled.

It makes no sense to have duplicate types in a union, unless they are
qualified (see [Qualifiers](Qualifiers.md)): `Str | Str` is equivalent to
`Str`, and simplified to it.

The type of a variable can never get more general (although its qualifiers can
change; see [Qualifiers](Qualifiers.md)), and Salvo does not support variable
shadowing. In the example above, `string_or_number` can move between `Str` and
`Int` only because its type *started* as a union.

### Testing a call

The subject of `is` can be a call rather than a variable, and then it is
evaluated **exactly once**: once per iteration, for a `while`. That is what
makes the take-until-empty loop the natural way to drain a collection:

```salvo
while remove_first(queue) is Ticket next {
    redeem(next)                    // one `remove_first` per turn
}
```

Because the binding and the test are two reads of one subject, a call subject
is allowed only where that single evaluation has somewhere to live: as the
whole condition of a `while`, or of an `if`'s first branch. Inside a `&&`
chain or an `elif` condition it is an error naming the remedy, which is to
bind it with `let` first, since hoisting it there would run it when
short-circuiting says it should not. A variable or a field chain is
unrestricted, because reading one twice costs nothing.

### Testing several arms at once

`is` with several types tests several arms of a union in one go: on an
`Int | Str | Bool`, `x is Int | Str` narrows `x` to `Int | Str`. It means the
tests joined with `||`, which narrows the same way, so it is written over a
variable or a field (bind a call's result first), and it binds nothing.

### A named union's functions

A union with a name, `type Shape = Circle | Square`, is a declaration of its
own, and it can declare functions in a body after the alias, exactly as a
struct does in its body:

```salvo
struct Circle { radius: Int }
struct Square { side: Int }

type Shape = Circle | Square : ToStr<self> {
    fn to_str(shape: Shape) -> Str {
        return when shape {
            is Circle { "a circle of ${shape.radius}" }
            is Square { "a square of ${shape.side}" }
        }
    }

    fn area(shape: proj Shape) -> Int {
        return when shape {
            is Circle { 3 * shape.radius * shape.radius }
            is Square { shape.side * shape.side }
        }
    }
}
```

Each one is an ordinary function, called bare or with dot notation
(`area(s)`, `s.area()`), that is **declared on the union**: it travels with
the type wherever it is imported, takes the type's visibility (an exported
union's functions are exported, and writing `export` on one is an error), and
`area@Shape` names it at a call or as a value. The obligation clause, when
there is one, stands between the alias and the body, and the body is one way
of meeting it, as for a struct ([Comparison and
hashing](Comparison-and-Hashing.md#declaring-them-on-your-own-type)). A
function on an arm's own type is a separate overload: above, a `Circle`
passed to `to_str` would still reach `Circle`'s own `to_str` if it had one,
and a `Shape` reaches the union's.

The body holds functions and [refinements](Qualifiers.md#where-refinements-live)
only, since a union's data is in its arms. Only a union has one: an alias
that names one type (`type Id = Int`) is another name for that type, with no
functions of its own, and a body on it is an error.

The name is still an alias ([Type aliases](Generics-and-Aliases.md#type-aliases)):
two named unions of the same arms are one type, so a function declared on one
is a function of the other too.

### Unions of literals

A literal is a type too: the type whose one value is that literal. A union of
them names a set of values, like a TypeScript string-literal union or an enum
without the ceremony:

```salvo
type Direction = "north" | "south" | "east" | "west"
type Retries = 0 | 1 | 3
```

Strings, integers (negative ones too), `Long`s (`3L`) and `Bool`s can be
literal types. A float cannot: equality on floats does not name a value.

A literal takes its literal type from where it is used and nowhere else.
`let d: Direction = "north"` is a `Direction`, a `"north"` passed to a
`Direction` parameter is one, and `let s = "north"` is a plain `Str`. A
literal the union does not list is an error that names the listed values, so a
typo is caught where it is written. A plain `Str` is not a `Direction` either,
since it could hold anything: narrow it first. `is` with several literals does
that in one test, comparing the value:

```salvo
fn direction(s: Str) [] -> Direction? => !s {
    if s is "north" | "south" | "east" | "west" {
        return s                         // a `Direction` here
    }
    return None
}
```

`is` tests a literal, and `when` over a union of literals is exhaustive, as
over any union:

```salvo
fn arrow(d: Direction) [] -> Str => d {
    when d {
        is "north" { return "↑" }
        is "south" { return "↓" }
        is "east" { return "→" }
        is "west" { return "←" }
    }
}
```

A union of literals widens to their base type wherever the base is expected: a
`Direction` passes where a `Str` is wanted, interpolates, and compares with
`==` against a literal. Unions compose, so a named sub-union is a group you can
test in one go:

```salvo
type Kms = "KmsDisabled" | "KmsThrottled"
type Code = Kms | "QueueDoesNotExist" | Other Str

if code is Kms { … }
```

### Open sets: `Other`

Many sets of values are open: a service may add a code this program has never
heard of. [`Other`](../../lib/std/core/other.sv#qualifier-other), a qualifier in
`std.core`, is the open arm: `"A" | "B" | Other Str` is the listed values and
any other string. A value reaches the `Other` arm only by being built with
[`other("…")`](../../lib/std/core/other.sv#fn-other), so everything else still
has to be one of the listed literals; `is Other` narrows to the rest.

### Literals at run time

**At run time a literal is its base.** A union of one base type's literals is
that base type on both backends (`Direction` is a `String`), and the `Other`
arm, being a `Str` too, collapses with them. In a union that mixes bases, each
base's literals share one arm: `"name" | 3 | "other" | Person` is a
`Str | Int | Person` at run time, its arms in the order they first appear.

Two consequences follow. Since two literals of one base are the same host type,
two overloads that differ only in literal types (`fn pick(x: "A")` and
`fn pick(x: "B")`) are refused; take the union in one function and `when` over
it. And `"a" | Str` is a warning, since every `Str` is already an arm; the
warning points at `Other Str`.

### Qualified literals

A literal can carry a qualifier like any other type:
`Short "port" | Short "verbose" | "host"`. The qualifier erases, so at run time
this is still one `Str`, and `is Short` is the test "the value is `"port"` or
`"verbose"`". Because the value alone decides, a union may not list one
literal twice under different qualifiers (`Short "port" | "port"`); that is an
error naming both arms.

## Structs

Data types can be combined into structs, which are like Kotlin classes whose
fields are all public and which have no methods:

```salvo
struct Person {
    name: Str,
    age: Int, // trailing comma is allowed but not required
}
```

A struct is instantiated by naming it and giving each field:

```salvo
let person: Person = Person {name: "Roland", age: 36}
```

When the type of a struct is known, the type name can be dropped from the
literal:

```salvo
let person: Person = {name: "Roland", age: 36}
```

A field whose value is a variable of the same name can be written by the name
alone, the inverse of destructuring (below). The type name is required for
this, since a bare `{ name }` is a set:

```salvo
let name = "Roland"
let age = 36
let person = Person {name, age}     // Person {name: name, age: age}
```

### Default field values

By default, every field of a struct must be given. To make one optional, give
it a default value:

```salvo
struct Person {
    name: Str,
    surname: Str | None = None, // surname is optional and nullable (see below)
    age: Int
}
```

Function parameters can give a default the same way; see
[Default parameter values](Functions.md#default-parameter-values).

A default must have the field's type. The one thing it may lack is a predicate
qualifier with constant arguments, such as
[`InRange(1, 65535)`](../../lib/std/core/range.sv#qualifier-inrange)
([Qualifiers](Qualifiers.md)): `port: InRange(1, 65535) Int = 8080` is
accepted, and the claim is tested each time the default is used, so a default
that does not satisfy it stops the program at that literal with a message
naming the field. Parameter defaults and a handler's state initialisers follow
the same rule.

### Destructuring and spreading

A struct can be destructured. When the variable has the same name as the
field, the field is mentioned alone; otherwise write `field: variable_name`:

```salvo
// `name` is type `Str`
// `their_age` is type `Int`
let {name, age: their_age} = person
```

A struct can be built from another with the spread operator `...`, which
re-uses the other's values and overrides the ones written after it:

```salvo
let person = Person {name: "Roland", age: 36}

// `person2` has the same name, but a different surname and age
let person2 = Person {...person, surname: "Elliott", age: 25}
```

Spreading a variable consumes it, since its fields now live in the new struct,
so `person` can no longer be used after building `person2`. Spread
`copy(person)` instead to keep both (see
[Deductions and ownership](Deductions-and-Ownership.md) for the full rules).

### Recursive structs

A struct may not **contain itself**, directly or through other structs: a value
of it would be infinitely large, so the declaration is an error naming the
field that closes the cycle.

```salvo
struct Node {
    value: Int,
    next: Node?      // error: `Node` contains itself through field `next`
}
```

Indirect through a container instead, which is how a tree or a linked structure
is written. A `List` holds any number of them, including none:

```salvo
struct Tree {
    value: Int,
    kids: List<Tree>     // fine, and so is `Tree[]`
}
```

Only edges that lay a value out *inline* count: a field of struct type, a
tuple element, a union arm, a nullable's inner type, an alias expansion, and a
type argument that a generic struct stores inline (`Wrapper<Looped>` where
`Wrapper<T>` has a `value: T`). A `List`, `Set`, `Map`, array, function type
or `proj` field already indirects, so recursion through one is legal,
including a mutually recursive pair (`Dir` holding `List<File>` and `File`
holding `List<Dir>`).

## Optional values: `T?`

Salvo has no null value. It emulates one with a union that has `None` as an
arm, and since a union of a single type `T` with `None` is so common, `T?` is
shorthand for `T | None`:

```salvo
let person: Person = Person {name: "Roland", age: 36}

// Type of `surname` is `Str | None`
let surname: Str? = person.surname
```

Such a value is called _nullable_, because the syntax resembles nullability in
Kotlin, and it is checked the same way as in Kotlin:

```salvo
// Because `Str?` is just `Str | None` we can use normal union type checking.
// The check narrows the field itself: inside the block, `person.surname`
// reads as `Str`.
if (person.surname is Str) {
    println("Surname is ${person.surname}") // String interpolation like in Kotlin
}
```

Everything on this page about unions applies to optionals. On top of that,
Salvo has Kotlin's operators for them: `?:` for a default or an early exit,
`?.` to reach through, and `?>` to pipe through. Each of them works on any
union, not only on optionals, by treating the union's **first arm** as "the
value" and the others as "the rest".

```salvo
fn greet(p: Person) -> Str {
    let nick: Str = p.nickname ?: "friend"
    return "hello ${nick}"
}
```

### Narrowing a place

Narrowing works for any chain of fields and tuple positions out of a variable
(`a.b.0.c`). The fact is dropped as soon as anything could have changed the
value: an assignment to that place or to one containing it, a reassignment of
the variable, or a call that takes it as `Mut`. Array elements do not narrow,
since `numbers[i]` cannot be told apart from `numbers[j]`, so for those, bind
the checked value with `is Str name` as shown for unions above.

Narrowing changes how a place reads, not where it is: writing a narrowed
variable writes the variable. After `if count is Int { count++ }`, `count`
holds the stepped value, below the branch as well as in it. A binding made
with `is Int n` is a variable of its own, so writing `n` leaves `count` as it
was.

```salvo
fn start() [] -> Int? {
    return 12
}

fn main() [use] -> None {
    use StdConsole()
    let count = start()
    if count is Int {
        count++
    }
    println("${count ?: 0}")        // 13
    if count is Int n {
        n = n * 2
        println("${n}")             // 26
    }
    println("${count ?: 0}")        // still 13
}
```

### Guards

A narrowing also survives a **guard**: when every branch of an `if` leaves the
block (by `return`, `break`, `continue`, or a call that never comes back),
what follows is on the else-path, so it reads with the remaining arms.

```salvo
fn next(p: Mut ListIter<T>) -> Emitted T | Finished => p: Mut {
    let element = get(p.items, p.at)
    if element is None {
        return finished()       // the only way out for the empty case
    }
    p.at = p.at + 1
    return emitted(element)     // `element` is `T` here, not `T?`
}
```

An `elif` chain accumulates the same way, so two exiting branches leave the
third arm. More generally, what follows an `if` or a `when` is on one of the
paths that fall through, so a place every one of them narrows reads below as
the union of what they narrow it to. When only the first branch falls through,
that is its test:

```salvo
if x is Point {
    log("a point")
} else {
    return
}
draw(x)                         // `x` is a `Point` here
```

After a `when`, the subject is one of the arms whose branches fall through. A
place some path does not narrow is not narrowed below, and when no branch
leaves, the paths cover every case and nothing changes.

### The negative test: `!is`

A guard usually wants the *negative* test, and that is `!is`:

```salvo
fn describe(s: Str?) -> Str => !s {
    if s !is Str {
        return "nothing"
    }
    return s                    // `s` is `Str` here
}
```

`x !is Q` is exactly `!(x is Q)`, the same check spelled the way a guard reads,
so it works wherever an `is` does, `&&`/`||` and `when` heads included. It
binds nothing: a failed test tells you nothing about the value on the branch it
guards, so there is nothing to name.

### Defaults and early exits: `?:`

`?:` is the **first-arm pick**. Its value is the subject when the subject holds
its union's first arm; otherwise the right side runs. On an optional, `?:`
keeps the one value arm, as in `greet` above. It is the safe counterpart of
Rust's `unwrap`: a path instead of a panic.

The right side is an ordinary expression, so it can also *leave*, which is the
shape that removes the most code:

```salvo
fn shout(p: Person) -> Str? {
    let nick: Str = p.nickname ?: return None
    return "${nick}!"
}
```

A right side that leaves contributes nothing to the type, so `nick` above is a
plain `Str`.

The pick selects by **position** over the subject's declared arm order, not by
name: it keeps the first arm. On `Ok Int | Err Str`:

```salvo
fn twice(text: Str) -> Ok Int | Err Str {
    let n: Ok Int = parse(text) ?: return _   // keep the first arm, Ok Int
    return ok(n * 2)
}
```

`_` is **the arms the pick left behind**, tags and all: every arm but the
first. In `?: return _` on `Ok Int | Err Str`, `_` is the `Err Str`, so the
right side can hand it to something that expects an error. That is the one
place the placeholder is needed, and it reads only inside a pick's right-hand
side.

A pick is a `when` underneath: `s ?: rhs` means "when `s` is its first arm,
`s`; otherwise `rhs`, with `_` as `s`". Everything a `when` does follows:
narrowing, the checks on what each path moves, and the code a backend writes.

A pick whose right side leaves also **narrows its subject** below, because the
subject must have been the picked arm for the code to be running:

```salvo
let v: Str = t ?: return        // `t` reads as `Str` from here on
let r2: Ok Int = r ?: return    // `r` reads as `Ok Int`, the picked arm
```

A right side that yields a value narrows nothing, since that is the path where
the subject was not picked.

The rest of the rules:

* The subject is evaluated once, so it can be a call.
* A pick on a subject with only **one** arm, so that the right side is
  unreachable, is an error: the same dead-scaffolding refusal a throw-free
  `try` gets.
* **Widening is allowed**: a right side whose type is outside the subject's
  arms joins into the result, so `n ?: "none"` on an `Int?` is an `Int | Str`.
* The operator works on any union: on `Str | Int | None` it keeps the `Str`,
  and `_` is `Int | None`.
* Its precedence is Kotlin's: tighter than `is` and comparison, looser than
  arithmetic. It is **left-associative**, so `count ?: 0 > 3` reads as
  `(count ?: 0) > 3`, and `a ?: b ?: 0` is `(a ?: b) ?: 0`, each pick handling
  the arms the last one left behind.

`?:` may start a continuation line, so a long chain can wrap; the newline
brackets everything before it on the left:

```salvo
let value = read_value()
    ?: fallback()
    ?: default()
```

### Values the compiler cannot prove

To get a non-null value out when the compiler cannot prove it is there (the
equivalent of Rust's `unwrap`), give `?:` a right side that stops the program:

```salvo
let person: Person = Person {name: "Roland", surname: "Elliott", age: 36}

// Haven't done an explicit check, but we know `surname` is `Str` because we just constructed it
println("Surname is ${person.surname ?: unreachable!()}")
```

### Reaching through: `?.`

The safe-navigation operator `?.` reaches through the **first arm** of a
union: `a?.b` performs `.b` on that arm and replaces it with the result,
passing the other arms through. For a `T?` that is the familiar reading:
`a?.b` is `None` when `a` is, and otherwise `a.b` lifted back into an
optional. `a?.m(args)` is the dot call `a.m(args)` on that arm: a member of the
arm if it has one, otherwise a function taking it first (see
[dot calls](Functions.md#dot-calls)). On a general union, `(A | B | C)?.field`, where
`A.field: F`, is `F | B | C`.

`?.` guards **one** link, so a chain spells every hop:
`notice.metadata.last_updated?.nanos` is a plain read of a non-optional
`last_updated`, while `o?.inner?.nanos` guards both. It pairs with `?:` for a
default (`last_updated?.nanos ?: -1L`), the two staying separate operators.
Like a pick, it is a `when` underneath: the receiver is evaluated once, so any
expression may stand before the `?.`, a call link included
(`find(id)?.label()`).

### Piping through: `?>`

To feed the first arm through an arbitrary expression the same way, the
null-safe pipe `?>` is the pipe analogue of `?.`: `number ?> to_str(_)` runs
`to_str` with `_` bound to the first-arm value and replaces that arm with the
result, passing the rest through. See
[Operators and the pipe](Control-Flow.md#operators-and-the-pipe).

### Comparing with `==`

An optional (or any union) may also be compared with `==` and `!=` against a
value whose type matches one of its arms: `x == -1L` on a `Long?` asks "is `x`
present and equal to -1", with `None`, or any other arm, comparing unequal. The
value's type picks the arm, so this works for any union, not just optionals.
It is `==` and `!=` only: ordering still needs the value narrowed first.
